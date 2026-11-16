#!/usr/bin/env python3
"""Compare actual unhinted paths using the same Bezier-extrema evaluator on both sides."""
import argparse
import ctypes as C
import hashlib
import json
from pathlib import Path
import subprocess
import fontTools
from fontTools.pens.boundsPen import BoundsPen
from fontTools.ttLib import TTFont
from compare_gdef_carets import HB


def bounds(commands, origin=(0, 0), flip_y=False):
    pen = BoundsPen(None)
    for c in commands:
        points = [(c[i]+origin[0], (-c[i+1] if flip_y else c[i+1])+origin[1]) for i in range(1,len(c),2)]
        if c[0] == 'M': pen.moveTo(points[0])
        elif c[0] == 'L': pen.lineTo(points[0])
        elif c[0] == 'Q': pen.qCurveTo(*points)
        elif c[0] == 'C': pen.curveTo(*points)
        elif c[0] == 'Z': pen.closePath()
    return pen.bounds


def union(boxes):
    boxes = [b for b in boxes if b is not None]
    return [min(b[0] for b in boxes), min(b[1] for b in boxes), max(b[2] for b in boxes), max(b[3] for b in boxes)] if boxes else None


def hb_commands(hb, gid):
    ptr = C.c_void_p
    funcs = hb.bind('hb_draw_funcs_create', ptr, [])()
    commands, callbacks = [], []
    for name, letter, count in [('move_to','M',2), ('line_to','L',2), ('quadratic_to','Q',4), ('cubic_to','C',6), ('close_path','Z',0)]:
        signature = C.CFUNCTYPE(None, ptr, ptr, ptr, *([C.c_float]*count), ptr)
        def collect(*args, letter=letter): commands.append([letter,*args[3:-1]])
        callback = signature(collect)
        callbacks.append(callback)
        hb.bind('hb_draw_funcs_set_'+name+'_func', None, [ptr, signature, ptr, ptr])(funcs, callback, None, None)
    hb.bind('hb_font_draw_glyph', None, [ptr, C.c_uint32, ptr, ptr])(hb.font, gid, funcs, None)
    hb.bind('hb_draw_funcs_destroy', None, [ptr])(funcs)
    return commands


def run(probe, path, size, direction, text):
    core = json.loads(subprocess.check_output([str(probe),str(path),'0',str(size),direction,text],text=True))
    for g in core['glyphs']:
        g['actual_bounds'] = union(bounds(l['commands'],l['origin']) for l in g.pop('layers'))
    return core


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--before',type=Path,required=True)
    p.add_argument('--after',type=Path,required=True)
    p.add_argument('--font',type=Path,action='append',required=True)
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args()
    rows=[]
    for path in a.font:
        font=TTFont(path)
        h=font['hhea']; size=h.ascent-h.descent+h.lineGap
        cmap=font.getBestCmap()
        hb=HB(path,0,font['head'].unitsPerEm)
        try:
            for text in ['A','Å','fi','\u0eb1','\u20d0','\u2422','x\u20d0']:
                if any(ord(c) not in cmap for c in text): continue
                for direction,code in [('ltr',4),('ttb',6)]:
                    shaped=hb.shape(text,code)
                    for g in shaped:
                        g['actual_bounds']=bounds(hb_commands(hb,g['glyph_id']),
                            (g['pen_xy'][0]+g['x_offset'],g['pen_xy'][1]-g['y_offset']), True)
                        name=font.getGlyphName(g['glyph_id'])
                        glyph=font['glyf'][name]
                        g.update(name=name,xmin=getattr(glyph,'xMin',None),lsb=font['hmtx'][name][1],contours=glyph.numberOfContours)
                    before=run(a.before,path,size,direction,text)
                    after=run(a.after,path,size,direction,text)
                    row=dict(font=path.name,sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                        text=text,codepoints=[f'U+{ord(c):04X}' for c in text],direction=direction,
                        harfbuzz=shaped,before=before,after=after,glyph_identity_verified=False)
                    for key,core in [('before',before),('after',after)]:
                        comparable=len(core['glyphs'])==len(shaped) and all(
                            x['source']==y['source'] and all(abs(u-v)<0.01 for u,v in zip(x['advance'],y['advance_xy']))
                            for x,y in zip(core['glyphs'],shaped))
                        if not comparable:
                            row[key+'_classification']='shaping_not_comparable'
                            continue
                        deltas=[]
                        for x,y in zip(core['glyphs'],shaped):
                            if x['actual_bounds'] is None or y['actual_bounds'] is None:
                                deltas.append(None)
                            else: deltas.append([u-v for u,v in zip(x['actual_bounds'],y['actual_bounds'])])
                        row[key+'_bounds_delta']=deltas
                        row[key+'_classification']='necessary_geometry_agrees_identity_unverified' if all(
                            (d is not None and max(map(abs,d))<=0.51) or
                            (d is None and x['actual_bounds'] is None and y['actual_bounds'] is None)
                            for d,x,y in zip(deltas,core['glyphs'],shaped)) else 'positioned_path_bounds_differ'
                    rows.append(row)
        finally:
            hb.destroy_font(hb.font);font.close()
    a.output.write_text(json.dumps(dict(harfbuzz_version=hb.version, fonttools_version=fontTools.__version__,
        before_probe_sha256=hashlib.sha256(a.before.read_bytes()).hexdigest(),
        after_probe_sha256=hashlib.sha256(a.after.read_bytes()).hexdigest(),
        evaluator='fontTools BoundsPen, true quadratic/cubic extrema on both actual drawable paths; y-down; 0.51 design-unit tolerance',
        before_revision='d98f6583c44a5946adab50dae786e9e4f4b00d22',after_revision='working tree static simple glyf metric-origin correction',rows=rows),indent=2,ensure_ascii=False)+'\n')

if __name__=='__main__': main()
