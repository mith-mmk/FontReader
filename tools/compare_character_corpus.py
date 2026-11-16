#!/usr/bin/env python3
"""Opt-in, horizontal real-font geometry comparison. Requires fontTools and libharfbuzz.
No downloads. Not a BiDi implementation, decoder, exhaustive IPA inventory or CI gate.
"""
import argparse
import ctypes as C
import ctypes.util
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import unicodedata as U
from fontTools.ttLib import TTFont


def corpus():
    rows = []
    provenance = {}
    # Arabic was tested separately. 12 is not an ISO-8859 part.
    for part in [1, 2, 3, 4, 5, 7, 8, 9, 10, 11, 13, 14, 15, 16]:
        for byte in range(160, 256):
            try:
                text = bytes([byte]).decode(f'iso8859_{part}')
            except UnicodeDecodeError:
                continue
            if U.category(text).startswith('C'):
                continue
            provenance.setdefault(text, []).append([part, byte])
    for text, origins in sorted(provenance.items(), key=lambda p: ord(p[0])):
        rows.append(dict(id=f'iso-{ord(text):04X}', text=text, group='iso-scalar',
                         direction='rtl' if U.bidirectional(text) in ('R', 'AL') else 'ltr',
                         locale='he' if 0x590 <= ord(text) <= 0x5ff else 'und', origins=origins))
    # Assigned IPA Extensions and Spacing Modifier Letters: coverage sweep only.
    for cp in range(0x250, 0x300):
        text = chr(cp)
        if U.category(text) == 'Cn':
            continue
        rows.append(dict(id=f'ipa-{cp:04X}', text=text, group='ipa-scalar', direction='ltr', locale='und'))
    sequences = [
        ('ipa-voiceless', 'n\u0325', 'und'), ('ipa-syllabic', 'n\u0329', 'und'),
        ('ipa-stacked', 'a\u0303\u0301', 'und'), ('ipa-two-sided', 'a\u0325\u0303', 'und'),
        ('ipa-tie', 't\u0361s', 'und'), ('ipa-aspirated', 't\u02b0', 'und'),
        ('ipa-length', '\u0251\u02d0', 'und'), ('ipa-stress', '\u02c8\u025b', 'und'),
        ('latin-acute', '\u00e9', 'fr'), ('latin-ring', '\u00e5', 'sv'),
        ('latin-caron', '\u0161', 'cs'), ('latin-double-acute', '\u0151', 'hu'),
        ('latin-umlaut', '\u00fc', 'de'), ('latin-vietnamese', '\u1eaf', 'vi'),
        ('hebrew-shalom', '\u05e9\u05c1\u05b8\u05dc\u05d5\u05b9\u05dd', 'he'),
        ('hebrew-dagesh', '\u05d1\u05bc\u05b7', 'he'),
        ('hebrew-stack', '\u05d0\u05b7\u0591', 'he'),
        ('hebrew-word', '\u05e9\u05dc\u05d5\u05dd', 'he'),
    ]
    for name, text, locale in sequences:
        for form in ('NFC', 'NFD'):
            rows.append(dict(id=f'{name}-{form}', text=U.normalize(form, text), group='sequence',
                             direction='rtl' if locale == 'he' else 'ltr', locale=locale))
    return rows


class HB:
    class Info(C.Structure):
        _fields_ = [(n, C.c_uint32) for n in ('codepoint', 'mask', 'cluster', 'var1', 'var2')]
    class Position(C.Structure):
        _fields_ = [(n, C.c_int32) for n in ('xa', 'ya', 'xo', 'yo', 'var')]
    class Extents(C.Structure):
        _fields_ = [(n, C.c_int32) for n in ('xb', 'yb', 'w', 'h')]

    def __init__(self, path, upem):
        lib = ctypes.util.find_library('harfbuzz')
        if not lib:
            raise RuntimeError('libharfbuzz not found')
        self.lib = C.CDLL(lib)
        def bind(name, result, args):
            f = getattr(self.lib, name); f.restype = result; f.argtypes = args
            return f
        ptr = C.c_void_p
        self.version = bind('hb_version_string', C.c_char_p, [])().decode()
        blob = bind('hb_blob_create_from_file_or_fail', ptr, [C.c_char_p])(str(path).encode())
        if not blob:
            raise RuntimeError('HarfBuzz cannot open font')
        face = bind('hb_face_create', ptr, [ptr, C.c_uint])(blob, 0)
        self.font = bind('hb_font_create', ptr, [ptr])(face)
        bind('hb_ot_font_set_funcs', None, [ptr])(self.font)
        bind('hb_font_set_scale', None, [ptr, C.c_int, C.c_int])(self.font, upem, upem)
        bind('hb_face_destroy', None, [ptr])(face)
        bind('hb_blob_destroy', None, [ptr])(blob)
        self.create = bind('hb_buffer_create', ptr, [])
        self.add = bind('hb_buffer_add_utf8', None, [ptr, C.c_char_p, C.c_int, C.c_uint, C.c_int])
        self.direction = bind('hb_buffer_set_direction', None, [ptr, C.c_int])
        self.language = bind('hb_language_from_string', ptr, [C.c_char_p, C.c_int])
        self.set_language = bind('hb_buffer_set_language', None, [ptr, ptr])
        self.guess = bind('hb_buffer_guess_segment_properties', None, [ptr])
        self.shape_fn = bind('hb_shape', None, [ptr, ptr, ptr, C.c_uint])
        self.infos = bind('hb_buffer_get_glyph_infos', C.POINTER(self.Info), [ptr, C.POINTER(C.c_uint)])
        self.positions = bind('hb_buffer_get_glyph_positions', C.POINTER(self.Position), [ptr, C.POINTER(C.c_uint)])
        self.extents = bind('hb_font_get_glyph_extents', C.c_int, [ptr, C.c_uint32, C.POINTER(self.Extents)])
        self.destroy = bind('hb_buffer_destroy', None, [ptr])
        self.destroy_font = bind('hb_font_destroy', None, [ptr])

    def shape(self, row):
        buf = self.create()
        try:
            text = row['text'].encode()
            self.add(buf, text, len(text), 0, len(text))
            self.direction(buf, 5 if row['direction'] == 'rtl' else 4)
            self.set_language(buf, self.language(row['locale'].encode(), -1))
            self.guess(buf)
            self.shape_fn(self.font, buf, None, 0)
            count = C.c_uint()
            info = self.infos(buf, C.byref(count)); pos = self.positions(buf, C.byref(count))
            x = 0; glyphs = []
            for i in range(count.value):
                ext = self.Extents(); p = pos[i]
                bounds = None
                if self.extents(self.font, info[i].codepoint, C.byref(ext)) and (ext.w or ext.h):
                    bounds = [x+p.xo+ext.xb, -p.yo-ext.yb,
                              x+p.xo+ext.xb+ext.w, -p.yo-ext.yb-ext.h]
                glyphs.append(dict(advance=p.xa, bounds=bounds))
                x += p.xa
            return dict(advance=x, glyphs=glyphs)
        finally:
            self.destroy(buf)


def same(a, b):
    if abs(a['advance']-b['advance']) > .1 or len(a['glyphs']) != len(b['glyphs']):
        return False
    # RTL public APIs may order vectors differently: compare sorted final ink boxes.
    def geom(row):
        return sorted((g['advance'], tuple(g['bounds']) if g['bounds'] else ())
                      for g in row['glyphs'])
    for (aa, ab), (ba, bb) in zip(geom(a), geom(b)):
        if abs(aa-ba) > .1 or len(ab) != len(bb):
            return False
        # HB integer extents round composite outlines; fontcore retains floats.
        if any(abs(x-y) > .51 for x, y in zip(ab, bb)):
            return False
    return True



def ink_union(row):
    boxes = [g['bounds'] for g in row['glyphs'] if g['bounds']]
    if not boxes:
        return None
    return [min(b[0] for b in boxes), min(b[1] for b in boxes),
            max(b[2] for b in boxes), max(b[3] for b in boxes)]


def same_union(a, b):
    aa, bb = ink_union(a), ink_union(b)
    if aa is None or bb is None:
        return aa == bb
    return all(abs(x-y) <= .51 for x, y in zip(aa, bb))


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--corpus', choices=('character', 'european'), default='character')
    ap.add_argument('--font', required=True, type=Path)
    ap.add_argument('--probe', required=True, type=Path)
    ap.add_argument('--output', required=True, type=Path)
    args = ap.parse_args()
    with TTFont(args.font) as font:
        cmap = font.getBestCmap(); hhea = font['hhea']; upem = font['head'].unitsPerEm
        size = hhea.ascent-hhea.descent+hhea.lineGap
        font_version = font['name'].getDebugName(5)
    if args.corpus == 'european':
        from european_corpus import corpus as european_corpus
        rows = european_corpus()
    else:
        rows = corpus()
    selected = [r for r in rows if all(ord(ch) in cmap for ch in r['text'])]
    payload = ''.join(f"{r['id']}\t{r['direction']}\t{r['locale']}\t{' '.join(f'{ord(c):X}' for c in r['text'])}\n" for r in selected)
    proc = subprocess.run([str(args.probe.resolve()), str(args.font.resolve()), str(size)],
                          input=payload, text=True, capture_output=True, check=True)
    actual = [json.loads(line) for line in proc.stdout.splitlines()]
    if [r['id'] for r in actual] != [r['id'] for r in selected]:
        raise RuntimeError('probe did not return every requested row in order')
    hb = HB(args.font, upem)
    results = []
    try:
        for row, got in zip(selected, actual):
            expected = hb.shape(row)
            # fontcore RTL has a right-edge origin, HB a left-edge origin.
            if row['direction'] == 'rtl':
                for glyph in got['glyphs']:
                    if glyph['bounds']:
                        glyph['bounds'][0] += got['advance']
                        glyph['bounds'][2] += got['advance']
            results.append(dict(**row, matches=same(got, expected), ink_union_matches=same_union(got, expected), actual=got, harfbuzz=expected))
    finally:
        hb.destroy_font(hb.font)
    report = dict(font=args.font.name, sha256=hashlib.sha256(args.font.read_bytes()).hexdigest(),
                  font_version=font_version, harfbuzz=hb.version, python=platform.python_version(), unicode=U.unidata_version,
                  mapping_source=('Explicit European sequences; Python Unicode NFC/NFD' if args.corpus == 'european'
                                  else 'Python standard-library iso8859 codecs; not downloaded Unicode mapping tables'),
                  units_per_em=upem, fontcore_size=size, cases=len(rows), cmap_covered=len(selected),
                  matches=sum(r['matches'] for r in results),
                  ink_union_matches=sum(r['ink_union_matches'] for r in results),
                  missing=[r['id'] for r in rows if r not in selected], results=results)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps({k:v for k,v in report.items() if k not in ('results','missing')}, indent=2))

if __name__ == '__main__':
    main()
