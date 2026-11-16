#!/usr/bin/env python3
"""Controlled, unhinted static glyf bearing diagnostic; writes copies only to --scratch."""
import argparse
import ctypes as C
import hashlib
import json
from pathlib import Path
import subprocess
from fontTools.ttLib import TTFont
from compare_gdef_carets import HB


def draw_points(hb, gid):
    ptr = C.c_void_p
    funcs = hb.bind('hb_draw_funcs_create', ptr, [])()
    points, callbacks = [], []
    for name, count in [('move_to', 2), ('line_to', 2), ('quadratic_to', 4), ('cubic_to', 6)]:
        signature = C.CFUNCTYPE(None, ptr, ptr, ptr, *([C.c_float]*count), ptr)
        def collect(*args):
            coords = args[3:-1]
            points.extend(zip(coords[::2], coords[1::2]))
        callback = signature(collect)
        callbacks.append(callback)
        hb.bind('hb_draw_funcs_set_'+name+'_func', None, [ptr, signature, ptr, ptr])(funcs, callback, None, None)
    hb.bind('hb_font_draw_glyph', None, [ptr, C.c_uint32, ptr, ptr])(hb.font, gid, funcs, None)
    hb.bind('hb_draw_funcs_destroy', None, [ptr])(funcs)
    return [min(p[0] for p in points), min(p[1] for p in points),
            max(p[0] for p in points), max(p[1] for p in points)]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--fixture', type=Path, required=True)
    p.add_argument('--probe', type=Path, required=True)
    p.add_argument('--scratch', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    args.scratch.mkdir(parents=True, exist_ok=True)
    rows = []
    for variant in ['original', 'lsb_equals_xmin', 'outline_xmin_zero', 'lsb_minus40']:
        font = TTFont(args.fixture, fontNumber=0, recalcTimestamp=False)
        glyphs = font['glyf']
        for name in font.getGlyphOrder():
            glyph = glyphs[name]
            if not glyph.numberOfContours:
                continue
            advance, lsb = font['hmtx'][name]
            if variant == 'lsb_equals_xmin':
                font['hmtx'][name] = advance, glyph.xMin
            elif variant == 'outline_xmin_zero':
                assert not glyph.isComposite(), 'diagnostic fixture must be simple'
                glyph.coordinates.translate((-glyph.xMin, 0))
                glyph.recalcBounds(glyphs)
            elif variant == 'lsb_minus40':
                font['hmtx'][name] = advance, -40
        path = args.scratch / (variant+'.ttf')
        font.save(path)
        hb = HB(path, 0, font['head'].unitsPerEm)
        h = font['hhea']
        size = h.ascent-h.descent+h.lineGap
        try:
            for direction, code, force in [('ltr', 4, False), ('ttb', 6, True)]:
                shaped = hb.shape('fi', code, force)
                assert len(shaped) == 1
                g = shaped[0]
                name = font.getGlyphName(g['glyph_id'])
                bounds = draw_points(hb, g['glyph_id'])
                core = json.loads(subprocess.check_output([str(args.probe), str(path), '0', str(size), direction, '-', 'fi'], text=True))
                rows.append(dict(variant=variant, direction=direction,
                    hb_features='liga=1' if force else 'default',
                    font_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                    head_flags=font['head'].flags, xmin=glyphs[name].xMin, xmax=glyphs[name].xMax, lsb=font['hmtx'][name][1],
                    hb_draw_glyph_unpositioned_y_up_bounds=bounds,
                    hb_positioned_draw_y_down_bounds=[bounds[0]+g['x_offset'], -bounds[3]-g['y_offset'], bounds[2]+g['x_offset'], -bounds[1]-g['y_offset']],
                    harfbuzz=g, hb_gdef_coordinates=hb.lookup(g['glyph_id'],code), fontcore=core))
        finally:
            hb.destroy_font(hb.font)
            font.close()
    args.output.write_text(json.dumps(dict(harfbuzz_version=hb.version,
        fixture_sha256=hashlib.sha256(args.fixture.read_bytes()).hexdigest(),
        scope='face0 static simple rectangular outlines; unhinted OT backend; draw control bounds exact for these rectangles only', rows=rows), indent=2)+'\n')

if __name__ == '__main__':
    main()
