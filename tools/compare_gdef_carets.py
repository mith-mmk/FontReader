#!/usr/bin/env python3
"""Opt-in GDEF caret diagnostics, no downloads. Requires fontTools and libharfbuzz.
Build examples/gdef_carets, then supply --probe and one or more --font paths.
HarfBuzz uses hb_ot_font_set_funcs, not FreeType/hinted contour callbacks.
"""
import argparse
import ctypes as C
import ctypes.util
import hashlib
from collections import Counter
import json
from pathlib import Path
import platform
import subprocess
import sys
import fontTools
from fontTools.ttLib import TTFont


class HB:
    class Info(C.Structure):
        _fields_ = [(n, C.c_uint32) for n in ('codepoint', 'mask', 'cluster', 'v1', 'v2')]
    class Position(C.Structure):
        _fields_ = [(n, C.c_int32) for n in ('xa', 'ya', 'xo', 'yo', 'var')]
    class Feature(C.Structure):
        _fields_ = [(n, C.c_uint32) for n in ('tag', 'value', 'start', 'end')]
    class Extents(C.Structure):
        _fields_ = [(n, C.c_int32) for n in ('xb', 'yb', 'w', 'h')]
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
        self.extents = self.bind('hb_font_get_glyph_extents', C.c_int,
            [ptr, C.c_uint32, C.POINTER(self.Extents)])
        self.v_origin = self.bind('hb_font_get_glyph_v_origin', C.c_int,
            [ptr, C.c_uint32, C.POINTER(C.c_int32), C.POINTER(C.c_int32)])
        self.destroy = self.bind('hb_buffer_destroy', None, [ptr])
        self.destroy_font = self.bind('hb_font_destroy', None, [ptr])

    def bind(self, name, result, args):
        function = getattr(self.lib, name)
        function.restype, function.argtypes = result, args
        return function

    def shape(self, text, direction, force_liga=False):
        buffer = self.create()
        try:
            data = text.encode()
            self.add(buffer, data, len(data), 0, len(data))
            self.direction(buffer, direction)
            self.guess(buffer)
            feature = self.Feature(int.from_bytes(b'liga', 'big'), 1, 0, 0xffffffff)
            self.shape_fn(self.font, buffer, C.byref(feature) if force_liga else None, int(force_liga))
            count = C.c_uint()
            infos = self.infos(buffer, C.byref(count))
            positions = self.positions(buffer, C.byref(count))
            output = []
            pen_x = pen_y = 0
            clusters = sorted(set(infos[i].cluster for i in range(count.value))) + [len(data)]
            for i in range(count.value):
                info, pos = infos[i], positions[i]
                extent = self.Extents()
                bounds = None
                if self.extents(self.font, info.codepoint, C.byref(extent)) and (extent.w or extent.h):
                    bounds = [pen_x+pos.xo+extent.xb, -(pen_y+pos.yo+extent.yb),
                              pen_x+pos.xo+extent.xb+extent.w, -(pen_y+pos.yo+extent.yb+extent.h)]
                vx, vy = C.c_int32(), C.c_int32()
                has_origin = bool(self.v_origin(self.font, info.codepoint, C.byref(vx), C.byref(vy)))
                output.append(dict(glyph_id=info.codepoint, cluster=info.cluster,
                    source=[info.cluster, clusters[clusters.index(info.cluster)+1]],
                    advance_xy=[pos.xa, -pos.ya], x_offset=pos.xo, y_offset=pos.yo,
                    pen_xy=[pen_x, -pen_y], bounds=bounds,
                    vertical_origin=[vx.value, vy.value] if has_origin else None))
                pen_x += pos.xa
                pen_y += pos.ya
            return output
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


def geometry_check(core, reference, direction):
    """Conservative necessary conditions, never proof of glyph identity."""
    a, b = core['glyphs'], reference
    if len(a) != len(b):
        return 'glyph_count_differs'
    a = sorted(a, key=lambda g: g['source'])
    b = sorted(b, key=lambda g: g['source'])
    rtl_shift = sum(g['advance_xy'][0] for g in a) if direction == 'rtl' else 0
    for ga, gb in zip(a, b):
        if ga['source'] != gb['source']:
            return 'source_clusters_differ'
        if any(abs(x-y) > .51 for x,y in zip(ga['advance_xy'], gb['advance_xy'])):
            return 'advances_differ'
        bounds = ga['bounds']
        if bounds is not None:
            bounds = [bounds[0]+rtl_shift, bounds[1], bounds[2]+rtl_shift, bounds[3]]
        if (bounds is None) != (gb['bounds'] is None):
            return 'ink_presence_differs'
        if bounds is not None and any(abs(x-y) > .51 for x,y in zip(bounds, gb['bounds'])):
            return 'positioned_ink_bounds_differ'
    return 'necessary_geometry_agrees_identity_unverified'


def normalized_carets(glyph, values, direction):
    # Format1/3 HB lookup is a glyph-coordinate scalar, not a glyph-origin-
    # adjusted point. TTB y_offset includes -v_origin plus GPOS placement.
    if direction == 'ltr':
        return [glyph['x_offset'] + v for v in values]
    if direction == 'rtl':
        return [glyph['advance_xy'][0] - glyph['x_offset'] - v for v in reversed(values)]
    return [-glyph['y_offset'] - v for v in reversed(values)]


def compare_font(path, face, probe, ppem=0, weight=None, fixture=False):
    font = TTFont(path, fontNumber=face)
    upem = font['head'].unitsPerEm
    hhea = font['hhea']
    size = hhea.ascent - hhea.descent + hhea.lineGap
    hb = HB(path, face, upem, ppem, weight)
    rows = []
    try:
        texts = (['ffi'] if face >= 7 else ['fi', 'fl']) if fixture else ['fi', 'fl', 'ff', 'ffi', 'ffl']
        for text in texts:
            for direction, code, force_liga in [('ltr', 4, False), ('rtl', 5, False),
                                                ('ttb', 6, False), ('ttb', 6, True)]:
                command = [str(probe), str(path), str(face), str(size), direction,
                           '-' if weight is None else str(weight), text]
                actual_hb = hb.shape(text, code, force_liga)
                for glyph in actual_hb:
                    name = font.getGlyphName(glyph['glyph_id'])
                    records = caret_records(font, name)
                    glyph['name'] = name
                    glyph['gdef'] = records
                    if 'glyf' in font:
                        outline = font['glyf'][name]
                        glyph['font_xmin'] = getattr(outline, 'xMin', None)
                        glyph['font_lsb'] = font['hmtx'][name][1]
                    values = hb.lookup(glyph['glyph_id'], code)
                    glyph['caret_coordinates'] = values
                    for record in records:
                        if record['format'] == 2:
                            x, y = C.c_int32(), C.c_int32()
                            record['harfbuzz_contour_callback_supported'] = bool(hb.contour_point(
                                hb.font, glyph['glyph_id'], record['point_index'], C.byref(x), C.byref(y)))
                    # Format2 HB coordinates are origin-adjusted already; this
                    # backend has no contour callback, so never normalize them.
                    if any(record['format'] == 2 for record in records):
                        glyph['normalized_candidates'] = None
                    else:
                        glyph['normalized_candidates'] = normalized_carets(glyph, values, direction)
                completed = subprocess.run(command, capture_output=True, text=True)
                row = dict(text=text, direction=direction, harfbuzz_shaping=actual_hb,
                    harfbuzz_feature_policy='explicit liga=1 (supplemental)' if force_liga else 'default',
                    reference_basis='actual requested-direction shaping; no LTR-glyph substitution',
                    harfbuzz_single_ligature=(len(actual_hb) == 1 and actual_hb[0]['source'] == [0,len(text)]),
                    glyph_identity_verified=False)
                if completed.returncode:
                    row.update(fontcore_error=completed.stderr.strip(), comparison='fontcore_error_not_comparable')
                    rows.append(row)
                    continue
                core = json.loads(completed.stdout)
                core_single = len(core['glyphs']) == 1 and core['glyphs'][0]['source'] == [0,len(text)]
                row.update(fontcore=core, fontcore_single_ligature=core_single,
                    fontcore_internal_stop_sources=dict(Counter(s['source'] for s in core['stops'][1:-1])),
                    shaping_geometry=geometry_check(core, actual_hb, direction))
                row['ligature_relation'] = ('single_ligature_both' if core_single and row['harfbuzz_single_ligature']
                    else 'fontcore_only' if core_single else 'harfbuzz_only' if row['harfbuzz_single_ligature']
                    else 'neither_single_ligature')
                row['gdef_formats_on_actual_hb_glyphs'] = sorted(set(
                    record['format'] for glyph in actual_hb for record in glyph['gdef']))
                row['fontcore_candidate_state'] = ('present' if any(g['candidates'] is not None for g in core['glyphs'])
                    else 'absent_or_unsupported_or_rejected')
                if core_single and row['harfbuzz_single_ligature']:
                    candidate, reference = core['glyphs'][0]['candidates'], actual_hb[0]['normalized_candidates']
                    row['fontcore_authored_adopted'] = bool(core['stops'][1:-1]) and all(
                        stop['source'] == 'FontAuthored' for stop in core['stops'][1:-1])
                    if candidate is not None and reference is not None:
                        equal = len(candidate) == len(reference) and all(abs(a-b) <= .51 for a,b in zip(candidate, reference))
                        row['raw_candidate_diagnostic'] = ('coordinates_agree_not_shaping_validation' if equal
                            else 'coordinates_differ_not_shaping_validation')
                    if fixture and row['shaping_geometry'] == 'positioned_ink_bounds_differ':
                        row['fixture_geometry_note'] = 'glyf.xMin=20, hmtx.lsb=0; HB OT extents apply bearing adjustment; core outline bounds preserve xMin. Not waived.'
                if row['shaping_geometry'] != 'necessary_geometry_agrees_identity_unverified':
                    row['comparison'] = 'shaping_not_comparable'
                elif not (core_single and row['harfbuzz_single_ligature']):
                    row['comparison'] = 'no_single_ligature_not_applicable'
                elif not actual_hb[0]['gdef']:
                    row['comparison'] = 'no_gdef_carets_fallback'
                elif actual_hb[0]['normalized_candidates'] is None:
                    row['comparison'] = 'reference_contour_unsupported_not_comparable'
                elif core['glyphs'][0]['candidates'] is None:
                    row['comparison'] = 'fontcore_candidate_unsupported_or_rejected'
                else:
                    candidate, reference = core['glyphs'][0]['candidates'], actual_hb[0]['normalized_candidates']
                    equal = len(candidate) == len(reference) and all(abs(a-b) <= .51 for a,b in zip(candidate, reference))
                    row['comparison'] = ('candidate_coordinates_agree_identity_unverified' if equal
                                         else 'candidate_coordinates_differ')
                    row['fontcore_authored_adopted'] = all(s['source'] == 'FontAuthored' for s in core['stops'][1:-1])
                rows.append(row)
        return dict(font=path.name, sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                    face=face, upem=upem, fontcore_size=size, harfbuzz_ppem=ppem, weight=weight,
                    has_vertical_metrics=('vmtx' in font), harfbuzz_version=hb.version, rows=rows)
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
    output = dict(schema=2, platform=platform.platform(), fonttools=fontTools.__version__,
                  fontcore_commit=commit, production_tree_dirty=production_dirty,
                  probe_sha256=hashlib.sha256(args.probe.read_bytes()).hexdigest(),
                  fontcore_features='default (cff, layout)',
                  fontcore_ppem='not exposed; Device adjustment is unsupported',
                  reference_backend='HarfBuzz OT font funcs; not a rasterizer/hinted contour backend',
                  scope='Actual-direction LTR/RTL/TTB default plus explicit TTB liga=1 supplemental diagnostics; geometry is not glyph identity proof',
                  fonts=fonts)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, ensure_ascii=False, indent=2)+'\n')
    counts = {}
    for font in fonts:
        for row in font['rows']:
            counts[row['comparison']] = counts.get(row['comparison'], 0)+1
    print(json.dumps(counts, sort_keys=True))


def self_test():
    glyph = dict(advance_xy=[900, 0], x_offset=50, y_offset=-750)
    assert normalized_carets(glyph, [220], 'ltr') == [270]
    assert normalized_carets(glyph, [220], 'rtl') == [630]
    assert normalized_carets(glyph, [200, 500], 'ttb') == [250, 550]
    # Invalid font order must not be repaired by sorting during normalization.
    assert normalized_carets(glyph, [500, 200], 'ttb') == [550, 250]
    base = dict(source=[0, 2], advance_xy=[900, 0], bounds=[20, -500, 300, 0])
    assert geometry_check(dict(glyphs=[base]), [dict(base)], 'ltr').startswith('necessary_geometry')
    shifted = dict(base, bounds=[0, -500, 280, 0])
    assert geometry_check(dict(glyphs=[base]), [shifted], 'ltr') == 'positioned_ink_bounds_differ'
    assert geometry_check(dict(glyphs=[base]), [base, base], 'ltr') == 'glyph_count_differs'
    assert geometry_check(dict(glyphs=[base]), [dict(base, source=[0,1])], 'ltr') == 'source_clusters_differ'
    assert geometry_check(dict(glyphs=[base]), [dict(base, advance_xy=[800,0])], 'ltr') == 'advances_differ'
    rtl = dict(base, bounds=[-880, -500, -600, 0])
    assert geometry_check(dict(glyphs=[rtl]), [base], 'rtl').startswith('necessary_geometry')
    print('10 diagnostic assertions passed')


if __name__ == '__main__':
    if sys.argv[1:] == ['--self-test']:
        self_test()
    else:
        main()
