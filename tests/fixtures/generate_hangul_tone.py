"""Original MIT fixtures, based on bounded Hangul; no third-party outlines."""
from pathlib import Path
from fontTools.ttLib import TTCollection, newTable
from fontTools.fontBuilder import FontBuilder
from fontTools.ttLib.tables._f_v_a_r import Axis
import generate_bounded_hangul as base

fonts=[]
gpos='feature vert { pos [v t] <500 1000 0 -1000>; } vert;'
for index in range(7):
    features=base.common+(base.good if index else '')+gpos
    if index==3: features+='feature ccmp { sub tone by bad; } ccmp;'
    if index==6: features+='feature calt { sub L by .notdef; } calt;'
    if index==4: features+='feature vrt2 { sub tone by bad; } vrt2;'
    base.features.append(features)
    f=base.build(len(base.features)-1)
    fb=FontBuilder(font=f)
    fb.setupVerticalMetrics({n:(1000,100) for n in base.names})
    fb.setupVerticalHeader(ascent=500,descent=-500)
    for table in f['cmap'].tables:
        if table.isUnicode():
            table.cmap[0x302f]='tone'
            table.cmap[0xe004]='tone'
    if index==2:
        f['hmtx'].metrics['tone']=(0,0)
    if index==5:
        f['fvar']=newTable('fvar');axis=Axis();axis.axisTag='wght';axis.minValue=100;axis.defaultValue=400;axis.maxValue=900;axis.flags=0;axis.axisNameID=256
        f['fvar'].axes=[axis];f['fvar'].instances=[]
    fonts.append(f)
c=TTCollection();c.fonts=fonts;c.save(Path(__file__).with_name('hangul-tone.ttc'))
