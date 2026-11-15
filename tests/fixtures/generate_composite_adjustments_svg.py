"""Generate an MIT clipped/gradient SVG font using only Python's standard library.

Read the committed MIT vertical-metrics.ttf; retain all original table payloads.
Add one supported SVG glyph with local clip and user-space linear/radial paints.
"""
from pathlib import Path
import struct

root = Path(__file__).parent
source = (root / 'vertical-metrics.ttf').read_bytes()
tables = {}
for i in range(struct.unpack_from('>H', source, 4)[0]):
    tag, _, offset, length = struct.unpack_from('>4sIII', source, 12 + i * 16)
    tables[tag] = source[offset:offset + length]
document = b'''<svg xmlns="http://www.w3.org/2000/svg" id="glyph1" viewBox="0 0 1000 1000">
<defs>
<clipPath id="clip"><rect x="0" y="0" width="400" height="600"/></clipPath>
<linearGradient id="linear" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="400" y2="600"><stop offset="0" stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/></linearGradient>
<radialGradient id="radial" gradientUnits="userSpaceOnUse" cx="200" cy="300" r="200"><stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#000000"/></radialGradient>
</defs>
<rect x="0" y="0" width="400" height="600" clip-path="url(#clip)" fill="url(#linear)"/>
<rect x="100" y="100" width="200" height="200" fill="url(#radial)"/>
</svg>'''
# SVG header + one document index entry. Document offset is relative to index.
tables[b'SVG '] = struct.pack('>HIIHHHII', 0, 10, 0, 1, 1, 1, 14, len(document)) + document
head = bytearray(tables[b'head'])
head[8:12] = bytes(4)
tables[b'head'] = bytes(head)
count = len(tables)
power = 1 << (count.bit_length() - 1)
output = bytearray(source[:4] + struct.pack('>HHHH', count, power * 16, power.bit_length() - 1, count * 16 - power * 16))
output += bytes(16 * count)
head_offset = 0

def checksum(data):
    padded = data + bytes((-len(data)) % 4)
    return sum(struct.unpack('>' + 'I' * (len(padded) // 4), padded)) & 0xffffffff

for i, (tag, data) in enumerate(sorted(tables.items())):
    offset = len(output)
    if tag == b'head':
        head_offset = offset
    struct.pack_into('>4sIII', output, 12 + 16 * i, tag, checksum(data), offset, len(data))
    output += data + bytes((-len(data)) % 4)
struct.pack_into('>I', output, head_offset + 8, (0xb1b0afba - checksum(output)) & 0xffffffff)
(root / 'composite-adjustments-svg.ttf').write_bytes(output)
