"""Compare strict old Hangul + tone ink bounds to HarfBuzz. External font only.
Usage: python tools/compare_hangul_tones.py FONT PROBE_BINARY
Requires fontTools and uharfbuzz; no downloading or modifying the input font.
"""
import uharfbuzz as hb, subprocess, json, sys, hashlib
from fontTools.ttLib import TTFont
font=sys.argv[1]; exe=sys.argv[2]
data=open(font,'rb').read(); f=hb.Font(hb.Face(data)); meta=TTFont(font)
size=meta['hhea'].ascent-meta['hhea'].descent+meta['hhea'].lineGap
texts=['가ᇫ','ᄓᅡ','ᄀᅶ','ꥠힰퟋ','ꥠᅡᇫ','ᄀힰᇫ','가ퟋ']
rows=[]
for text in texts:
 for tone in ['〮','〯']:
  for direction in ['ltr','ttb']:
   t=text+tone;b=hb.Buffer();b.add_str(t);b.direction=direction;b.script='hang';b.language='ko';hb.shape(f,b)
   expected=[];x=y=0
   for i,p in zip(b.glyph_infos,b.glyph_positions):
    e=f.get_glyph_extents(i.codepoint);x0=x+p.x_offset+e.x_bearing;y0=-y-p.y_offset-e.y_bearing
    expected.append([x0,y0,x0+e.width,y0-e.height,p.x_advance,-p.y_advance]);x+=p.x_advance;y+=p.y_advance
   actual=[json.loads(line) for line in subprocess.check_output([exe,font,t,str(size)]+(['ttb'] if direction=='ttb' else []),text=True).splitlines()]
   rows.append(dict(text=t,codepoints=[f'U+{ord(c):04X}' for c in t],direction=direction,expected=expected,actual=actual,match=actual==expected))
print(json.dumps(dict(harfbuzz=hb.version_string(),font_sha256=hashlib.sha256(data).hexdigest(),font_size=size,rows=rows),ensure_ascii=False,indent=2))

sys.exit(0 if all(r["match"] for r in rows) else 1)
