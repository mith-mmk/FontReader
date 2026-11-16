"""Original MIT BiDi layout fixture; no external font outlines.

Regenerate with fontTools. All dimensions are in a 1000-unit em. Deliberately
visible, 777-unit controls detect accidental shaping of BiDi formatting chars.
Mirror pairs have equal advances but different asymmetric outlines.
"""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString

CONTROLS = [0x061C, 0x200E, 0x200F, *range(0x202A, 0x202F), *range(0x2066, 0x206A)]
CHARACTERS = ' ABabfi12אבג\u05b0ب()[]{}<>«»∈∋'
CODES = {ord(ch): f'u{ord(ch):04X}' for ch in CHARACTERS}
CODES.update({code: f'u{code:04X}' for code in CONTROLS})
ORDER = ['.notdef', *CODES.values(), 'fi', 'beh.isol', 'beh.init', 'beh.medi', 'beh.fina']
MIRRORS = '()[]{}<>«»∈∋'


def build():
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(ORDER)
    fb.setupCharacterMap(CODES)
    glyphs = {}
    metrics = {}
    for index, name in enumerate(ORDER):
        pen = TTGlyphPen(None)
        # Unique asymmetric contours distinguish mirrored alternatives and forms.
        if name != 'u0020':
            x = 20 + index * 3
            y = 650 if name == 'u05B0' else 0
            pen.moveTo((x, y))
            pen.lineTo((x + 90 + index, y))
            pen.lineTo((x + 30, y + 100 if name == 'u05B0' else 500))
            pen.closePath()
        glyphs[name] = pen.glyph()
        advance = 600
        if name == 'u0020':
            advance = 300
        elif name == 'u05B0':
            advance = 0
        elif name == 'fi':
            advance = 900
        elif name in {CODES[ord(ch)] for ch in MIRRORS}:
            advance = 400
        elif name in {CODES[code] for code in CONTROLS}:
            advance = 777
        metrics[name] = (advance, 0)
    fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics(metrics)
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    name = 'MITBidiLayout'
    fb.setupNameTable(dict(familyName=name, styleName='Regular',
                          uniqueFontIdentifier=name, fullName=name, psName=name))
    fb.setupOS2(sTypoAscender=800, sTypoDescender=-200,
                usWinAscent=800, usWinDescent=200)
    fb.setupPost()
    addOpenTypeFeaturesFromString(fb.font, '''
        languagesystem DFLT dflt;
        languagesystem latn dflt;
        languagesystem hebr dflt;
        languagesystem arab dflt;
        feature liga { sub u0066 u0069 by fi; } liga;
        feature isol { sub u0628 by beh.isol; } isol;
        feature init { sub u0628 by beh.init; } init;
        feature medi { sub u0628 by beh.medi; } medi;
        feature fina { sub u0628 by beh.fina; } fina;
        table GDEF { GlyphClassDef
            [u0041 u0042 u0061 u0062 u0066 u0069 u05D0 u05D1 u05D2 u0628
             beh.isol beh.init beh.medi beh.fina], [fi], [u05B0], [];
        } GDEF;
    ''')
    fb.font['head'].created = fb.font['head'].modified = 2082844800
    fb.font.recalcTimestamp = False
    return fb.font


if __name__ == '__main__':
    build().save(Path(__file__).with_name('bidi-layout.ttf'))
