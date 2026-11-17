#!/usr/bin/env python3
"""Opt-in Format 2 reference via installed HarfBuzz+FreeType; no downloads/builds."""
import argparse
import ctypes as C
import ctypes.util
import hashlib
import json
from fontTools.ttLib import TTFont
from compare_gdef_carets import HB, caret_records


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--fixture',required=True)
    p.add_argument('--output',required=True)
    p.add_argument('--face',type=int,default=1)
    p.add_argument('--glyph',action='append',help='known glyph name; defaults to fi and fl')
    a=p.parse_args()
    ft=C.CDLL(ctypes.util.find_library('freetype'))
    def bind(name,result,args):
        f=getattr(ft,name); f.restype=result; f.argtypes=args; return f
    ptr=C.c_void_p
    library,face=ptr(),ptr()
    init=bind('FT_Init_FreeType',C.c_int,[C.POINTER(ptr)])
    new=bind('FT_New_Face',C.c_int,[ptr,C.c_char_p,C.c_long,C.POINTER(ptr)])
    done_face=bind('FT_Done_Face',C.c_int,[ptr])
    done_lib=bind('FT_Done_FreeType',C.c_int,[ptr])
    assert init(C.byref(library))==0
    assert new(library,a.fixture.encode(),a.face,C.byref(face))==0
    major,minor,patch=C.c_int(),C.c_int(),C.c_int()
    bind('FT_Library_Version',None,[ptr,C.POINTER(C.c_int),C.POINTER(C.c_int),C.POINTER(C.c_int)])(library,C.byref(major),C.byref(minor),C.byref(patch))
    font=TTFont(a.fixture,fontNumber=a.face)
    hb=HB(a.fixture,a.face,font['head'].unitsPerEm)
    ot_font=hb.font
    ft_font=hb.bind('hb_ft_font_create_referenced',ptr,[ptr])(face)
    assert ft_font
    hb.bind('hb_font_set_scale',None,[ptr,C.c_int,C.c_int])(ft_font,font['head'].unitsPerEm,font['head'].unitsPerEm)
    # NO_HINTING | NO_BITMAP. Do not use NO_SCALE: hb-ft callbacks expect 26.6 scaled values.
    flags=2|8
    hb.bind('hb_ft_font_set_load_flags',None,[ptr,C.c_int])(ft_font,flags)
    origin_point=hb.bind('hb_font_get_glyph_contour_point_for_origin',C.c_int,[ptr,C.c_uint32,C.c_uint,C.c_int,C.POINTER(C.c_int32),C.POINTER(C.c_int32)])
    rows=[]
    try:
        for backend,handle in [('opentype_no_contour_callback',ot_font),('freetype_unhinted',ft_font)]:
            hb.font=handle
            for name in (a.glyph or ['fi','fl']):
                # Obtain known glyph IDs from the fixture, not from alternate-direction shaping.
                gid=font.getGlyphID(name)
                records=caret_records(font,name)
                for direction,code in [('ltr',4),('rtl',5),('ttb',6)]:
                    points=[]
                    for r in records:
                        i=r['point_index']; x,y=C.c_int32(),C.c_int32()
                        supported=bool(hb.contour_point(handle,gid,i,C.byref(x),C.byref(y)))
                        ox,oy=C.c_int32(),C.c_int32()
                        origin_supported=bool(origin_point(handle,gid,i,code,C.byref(ox),C.byref(oy)))
                        points.append(dict(index=i,raw_design_point=r.get('unhinted_contour_point'),
                            callback_supported=supported,callback_point=[x.value,y.value] if supported else None,
                            origin_callback_supported=origin_supported,origin_point=[ox.value,oy.value] if origin_supported else None))
                    vx,vy=C.c_int32(),C.c_int32()
                    hb.v_origin(handle,gid,C.byref(vx),C.byref(vy))
                    values=hb.lookup(gid,code)
                    rows.append(dict(backend=backend,glyph=name,glyph_id=gid,direction=direction,
                        points=points,vertical_origin=[vx.value,vy.value],gdef_query=values,
                        classification='supported_reference' if all(x['callback_supported'] for x in points) else 'unsupported_callback_not_a_zero_coordinate',
                        # Format2 query already uses the direction origin. TTB only changes sign to y-down.
                        ttb_pen_distance=[-v for v in values] if direction=='ttb' and all(x['callback_supported'] for x in points) else None))
    finally:
        hb.destroy_font(ft_font);hb.destroy_font(ot_font)
        done_face(face);done_lib(library);font.close()
    with open(a.output,'w') as f:
        json.dump(dict(harfbuzz_version=hb.version,freetype_version=f'{major.value}.{minor.value}.{patch.value}',
            fixture_sha256=hashlib.sha256(open(a.fixture,'rb').read()).hexdigest(),face=a.face,
            ft_load_flags=flags,ft_load_flags_names=['NO_HINTING','NO_BITMAP'],scale=[1000,1000],
            scope='direct known fixture glyph queries; not text shaping equivalence; no product implementation compared',rows=rows),f,indent=2)
        f.write('\n')

if __name__=='__main__': main()
