#!/usr/bin/env python3
"""Opt-in GDEF caret diagnostics, no downloads. Requires fontTools and libharfbuzz.
Build examples/gdef_carets, then supply --probe and one or more --font paths.
HarfBuzz uses hb_ot_font_set_funcs, not FreeType/hinted contour callbacks.
"""
import argparse
import ctypes as C
import ctypes.util
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import fontTools
from fontTools.ttLib import TTFont


class HB:
    class Info(C.Structure):
        _fields_ = [(n, C.c_uint32) for n in ('codepoint', 'mask', 'cluster', 'v1', 'v2')]
    class Position(C.Structure):
        _fields_ = [(n, C.c_int32) for n in ('xa', 'ya', 'xo', 'yo', 'var')]
    class Variation(C.Structure):
        _fields_ = [('tag', C.c_uint32), ('value', C.c_float)]

    def __init__(self, path, face_index, upem, ppem=0, weight=None):
        library = ctypes.util.find_library('harfbuzz')
        if not library:
            raise RuntimeError('libharfbuzz is required')
        self.lib = C.CDLL(library)
        ptr = C.c_void_p
        self.version = self.bind('hb_version_string', C.c_char_p, [])().decode()
        blob = self.bind('hb_blob_create_from_file_or_fail', ptr, [C.c_char_p])(str(path).encode())
        if not blob:
            raise RuntimeError('HarfBuzz cannot open font')
        face = self.bind('hb_face_create', ptr, [ptr, C.c_uint])(blob, face_index)
        self.font = self.bind('hb_font_create', ptr, [ptr])(face)
        self.bind('hb_ot_font_set_funcs', None, [ptr])(self.font)
        self.bind('hb_font_set_scale', None, [ptr, C.c_int, C.c_int])(self.font, upem, upem)
        self.bind('hb_font_set_ppem', None, [ptr, C.c_uint, C.c_uint])(self.font, ppem, ppem)
        if weight is not None:
            value = self.Variation(int.from_bytes(b'wght', 'big'), weight)
            self.bind('hb_font_set_variations', None, [ptr, C.POINTER(self.Variation), C.c_uint])(self.font, C.byref(value), 1)
        self.bind('hb_face_destroy', None, [ptr])(face)
        self.bind('hb_blob_destroy', None, [ptr])(blob)
        self.create = self.bind('hb_buffer_create', ptr, [])
        self.add = self.bind('hb_buffer_add_utf8', None, [ptr, C.c_char_p, C.c_int, C.c_uint, C.c_int])
        self.direction = self.bind('hb_buffer_set_direction', None, [ptr, C.c_int])
        self.guess = self.bind('hb_buffer_guess_segment_properties', None, [ptr])
        self.shape_fn = self.bind('hb_shape', None, [ptr, ptr, ptr, C.c_uint])
        self.infos = self.bind('hb_buffer_get_glyph_infos', C.POINTER(self.Info), [ptr, C.POINTER(C.c_uint)])
        self.positions = self.bind('hb_buffer_get_glyph_positions', C.POINTER(self.Position), [ptr, C.POINTER(C.c_uint)])
        self.carets = self.bind('hb_ot_layout_get_ligature_carets', C.c_uint,
            [ptr, C.c_int, C.c_uint32, C.c_uint, C.POINTER(C.c_uint), C.POINTER(C.c_int32)])
        self.contour_point = self.bind('hb_font_get_glyph_contour_point', C.c_int,
            [ptr, C.c_uint32, C.c_uint, C.POINTER(C.c_int32), C.POINTER(C.c_int32)])
        self.destroy = self.bind('hb_buffer_destroy', None, [ptr])
        self.destroy_font = self.bind('hb_font_destroy', None, [ptr])

    def bind(self, name, result, args):
        function = getattr(self.lib, name)
        function.restype, function.argtypes = result, args
        return function

    def shape(self, text, direction):
        buffer = self.create()
        try:
            data = text.encode()
            self.add(buffer, data, len(data), 0, len(data))
            self.direction(buffer, direction)
            self.guess(buffer)
            self.shape_fn(self.font, buffer, None, 0)
            count = C.c_uint()
            infos = self.infos(buffer, C.byref(count))
            positions = self.positions(buffer, C.byref(count))
            return [dict(glyph_id=infos[i].codepoint, cluster=infos[i].cluster,
                         advance=positions[i].xa, x_offset=positions[i].xo,
                         y_offset=positions[i].yo) for i in range(count.value)]
        finally:
            self.destroy(buffer)

    def lookup(self, glyph, direction):
        count = C.c_uint()
        total = self.carets(self.font, direction, glyph, 0, C.byref(count), None)
        values = (C.c_int32 * total)()
        count.value = total
        self.carets(self.font, direction, glyph, 0, C.byref(count), values)
        return list(values[:count.value])


def caret_records(font, glyph):
    table = font.get('GDEF')
    listing = getattr(table.table, 'LigCaretList', None) if table else None
    if not listing or glyph not in listing.Coverage.glyphs:
        return []
    records = listing.LigGlyph[listing.Coverage.glyphs.index(glyph)].CaretValue
    output = []
    for caret in records:
        row = dict(format=caret.Format)
        if caret.Format == 2:
            row['point_index'] = caret.CaretValuePoint
            if 'glyf' in font:
                points = font['glyf'][glyph].getCoordinates(font['glyf'])[0]
                if caret.CaretValuePoint < len(points):
                    row['unhinted_contour_point'] = list(points[caret.CaretValuePoint])
        else:
            row['coordinate'] = caret.Coordinate
            device = getattr(caret, 'DeviceTable', None)
            if device:
                row['adjustment'] = dict(format=device.DeltaFormat,
                    start_or_outer=device.StartSize, end_or_inner=device.EndSize,
                    deltas=getattr(device, 'DeltaValue', None))
        output.append(row)
    return output


def compare_font(path, face, probe, ppem=0, weight=None, fixture=False):
    font = TTFont(path, fontNumber=face)
    upem = font['head'].unitsPerEm
    hhea = font['hhea']
    # fontcore font_size is line-height based. This yields scale=1 design unit.
    size = hhea.ascent - hhea.descent + hhea.lineGap
    hb = HB(path, face, upem, ppem, weight)
    rows = []
    try:
        for text in ((['ffi'] if face >= 7 else ['fi', 'fl']) if fixture else ['fi', 'fl', 'ff', 'ffi', 'ffl']):
            # Exact LTR glyph ID gives a stable direction-independent GDEF query.
            # RTL shaping is recorded independently, not assumed equivalent.
            ltr = hb.shape(text, 4)
            for direction, code in [('ltr', 4), ('rtl', 5)]:
                command = [str(probe), str(path), str(face), str(size), direction,
                           '-' if weight is None else str(weight), text]
                core = json.loads(subprocess.check_output(command, text=True))
                actual_hb = hb.shape(text, code)
                row = dict(text=text, direction=direction, fontcore=core,
                           harfbuzz_shaping=actual_hb,
                           same_reference_glyph_selection=(len(ltr) == len(actual_hb) == 1 and ltr[0]['glyph_id'] == actual_hb[0]['glyph_id']))
                if len(ltr) == 1:
                    selected = ltr[0]['glyph_id']
                    name = font.getGlyphName(selected)
                    records = caret_records(font, name)
                    values = hb.lookup(selected, code)
                    row.update(reference_glyph=dict(id=selected, name=name),
                               gdef=records, harfbuzz_caret_coordinates=values,
                               reference_basis='glyph selected by HarfBuzz LTR; GDEF queried with requested direction')
                    for record in records:
                        if record['format'] == 2:
                            x, y = C.c_int32(), C.c_int32()
                            record['harfbuzz_contour_callback_supported'] = bool(hb.contour_point(
                                hb.font, selected, record['point_index'], C.byref(x), C.byref(y)))
                    if len(core['glyphs']) == 1 and abs(core['glyphs'][0]['advance'] - ltr[0]['advance']) <= .1:
                        glyph = core['glyphs'][0]
                        # HB carets are glyph-origin x coordinates, not RTL
                        # initial-pen distances; RTL initial pen is right edge.
                        normalized = sorted([ltr[0]['x_offset'] + value for value in values] if direction == 'ltr' else
                                            [ltr[0]['advance'] - ltr[0]['x_offset'] - value for value in values])
                        row['harfbuzz_initial_pen_distances'] = normalized
                        candidate = glyph['candidates']
                        if candidate is not None:
                            row['comparison'] = ('candidate_query_match' if len(candidate) == len(normalized)
                                and all(abs(a-b) <= .51 for a,b in zip(candidate, normalized))
                                else 'candidate_query_mismatch')
                        else:
                            row['comparison'] = 'fallback_or_absent'
                    else:
                        row['comparison'] = 'shaping_not_comparable'
                else:
                    row['comparison'] = 'no_single_ltr_ligature'
                rows.append(row)
        return dict(font=path.name, sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                    face=face, upem=upem, fontcore_size=size, harfbuzz_ppem=ppem, weight=weight,
                    harfbuzz_version=hb.version, rows=rows)
    finally:
        hb.destroy_font(hb.font)
        font.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--probe', type=Path, required=True)
    parser.add_argument('--font', type=Path, action='append', default=[])
    parser.add_argument('--fixture', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    fonts = [compare_font(path, 0, args.probe.resolve()) for path in args.font]
    if args.fixture:
        header = args.fixture.read_bytes()[:12]
        face_count = int.from_bytes(header[8:12], 'big')
        if header[:4] != b'ttcf' or not 1 <= face_count <= 256:
            raise ValueError('--fixture must be a TTC with 1..256 faces')
        for face in range(face_count):
            settings = [(0, None)]
            if face == 3: settings += [(20, None)]
            if face in (4, 5): settings += [(0, 650), (0, 900)]
            for ppem, weight in settings:
                fonts.append(compare_font(args.fixture, face, args.probe.resolve(), ppem, weight, True))
    repo = Path(__file__).resolve().parents[1]
    try:
        commit = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD'], text=True).strip()
        production_dirty = bool(subprocess.check_output(
            ['git', '-C', str(repo), 'status', '--porcelain', '--', 'src', 'Cargo.toml', 'Cargo.lock'], text=True))
    except (OSError, subprocess.CalledProcessError):
        commit, production_dirty = None, None
    output = dict(schema=1, platform=platform.platform(), fonttools=fontTools.__version__,
                  fontcore_commit=commit, production_tree_dirty=production_dirty,
                  probe_sha256=hashlib.sha256(args.probe.read_bytes()).hexdigest(),
                  fontcore_features='default (cff, layout)',
                  fontcore_ppem='not exposed; Device adjustment is unsupported',
                  reference_backend='HarfBuzz OT font funcs; not a rasterizer/hinted contour backend',
                  scope='Horizontal GDEF diagnostics; no pixel, GUI, paragraph BiDi or vertical conformance claim',
                  fonts=fonts)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, ensure_ascii=False, indent=2)+'\n')
    counts = {}
    for font in fonts:
        for row in font['rows']:
            counts[row['comparison']] = counts.get(row['comparison'], 0)+1
    print(json.dumps(counts, sort_keys=True))


if __name__ == '__main__':
    main()
