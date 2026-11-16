"""Original MIT vertical Hangul fixtures. fontTools 4.68.0; no external outlines."""
from pathlib import Path
from copy import deepcopy
from fontTools.fontBuilder import FontBuilder
from fontTools.ttLib import TTCollection
from fontTools.ttLib.tables import otTables
import generate_bounded_hangul as base

gpos='feature vert { pos [v t] <500 1000 0 -1000>; } vert;'
shared=base.common+'''lookup Shared { sub L by l; sub l by bad; } Shared;
feature ljmo { lookup Shared; } ljmo;
feature vjmo { lookup V1; } vjmo;
feature tjmo { lookup T1; } tjmo;
feature vrt2 { lookup Shared; } vrt2;'''
foreign='feature vrt2 { script latn; sub L by A; } vrt2;'
forms=[
base.common+gpos,
base.common+base.good+'feature vert { sub l by bad; } vert;'+gpos,
base.common+base.good+'feature vert { sub l by A; } vert; feature vrt2 { sub l by bad; } vrt2;'+gpos,
shared+gpos,
base.common+'feature vrt2 { sub L by A; } vrt2;'+base.good+gpos,
base.common+base.good+'feature vert { sub l by bad; } vert;'+foreign+gpos,
base.common+foreign+gpos,
base.common+base.good+'feature vert { sub l by A; } vert; feature vrt2 { sub l by bad; } vrt2;'+gpos,
base.common+base.good+'feature vert { sub l by .notdef; } vert;'+gpos,
base.common+base.good+'feature vert { sub l by bad bad; } vert;'+gpos,
base.common+base.good+'feature vrt2 { sub l by bad; } vrt2;',
base.common+base.good+'feature liga { sub l by A; } liga; feature vrt2 { sub l by bad; } vrt2;'+gpos,
base.common+base.good+'feature liga { sub A by ga; } liga; feature vrt2 { sub l by bad; } vrt2;'+gpos,
base.common+base.good+'feature calt { sub L by .notdef; } calt; feature vrt2 { sub l by bad; } vrt2;'+gpos,
base.common+base.good+'feature liga { sub l by l; } liga; feature vrt2 { sub l by bad; } vrt2;'+gpos,
base.common+'lookup Shared { sub L by l; } Shared; feature ljmo { lookup Shared; } ljmo; feature liga { lookup Shared; } liga; feature vjmo { lookup V1; } vjmo; feature tjmo { lookup T1; } tjmo; feature vrt2 { sub l by bad; } vrt2;'+gpos,
base.common+base.good+"feature liga { sub l' lookup L1 v t; } liga; feature vrt2 { sub l by bad; } vrt2;"+gpos,
]
fonts=[]
for number,fea in enumerate(forms):
 index=len(base.features);base.features.append(fea);font=base.build(index)
 fb=FontBuilder(font=font);fb.setupVerticalMetrics({n:(1000,100) for n in base.names});fb.setupVerticalHeader(ascent=500,descent=-500)
 if number==7:
  table=font['GSUB'].table; script=table.ScriptList.ScriptRecord[0].Script
  lang=otTables.LangSysRecord();lang.LangSysTag='KOR ';lang.LangSys=deepcopy(script.DefaultLangSys)
  lang.LangSys.FeatureIndex=[i for i in lang.LangSys.FeatureIndex if table.FeatureList.FeatureRecord[i].FeatureTag!='vrt2'];lang.LangSys.FeatureCount=len(lang.LangSys.FeatureIndex)
  script.LangSysRecord=[lang];script.LangSysCount=1
 fonts.append(font)
collection=TTCollection();collection.fonts=fonts;collection.save(Path(__file__).with_name('bounded-vertical-hangul.ttc'))
