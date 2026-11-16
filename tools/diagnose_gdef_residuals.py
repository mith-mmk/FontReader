#!/usr/bin/env python3
"""Read-only RTL experiment; reuse the comparison harness's HB bindings.
No downloads and no product changes. fontTools + native HarfBuzz required.
"""
import argparse
import ctypes as C
import hashlib
import json
from pathlib import Path
import subprocess
from fontTools.ttLib import TTFont
from compare_gdef_carets import HB


def shape(hb, text, direction, script, features):
    buffer = hb.create()
    try:
        data = text.encode()
        hb.add(buffer, data, len(data), 0, len(data))
        hb.direction(buffer, direction)
        if script != 'guess':
            hb.bind('hb_buffer_set_script', None, [C.c_void_p, C.c_uint32])(
                buffer, int.from_bytes(script.encode(), 'big'))
        language = hb.bind('hb_language_from_string', C.c_void_p, [C.c_char_p, C.c_int])(b'en', -1)
        hb.bind('hb_buffer_set_language', None, [C.c_void_p, C.c_void_p])(buffer, language)
        hb.guess(buffer)
        actual_script = hb.bind('hb_buffer_get_script', C.c_uint32, [C.c_void_p])(buffer)
        values = (hb.Feature * len(features))(*[
            hb.Feature(int.from_bytes(tag.encode(), 'big'), value, 0, 0xffffffff)
            for tag, value in features.items()])
        hb.shape_fn(hb.font, buffer, values if features else None, len(features))
        count = C.c_uint()
        infos = hb.infos(buffer, C.byref(count))
        return dict(script=actual_script.to_bytes(4, 'big').decode(), language='en',
                    glyph_ids=[infos[i].codepoint for i in range(count.value)],
                    clusters=[infos[i].cluster for i in range(count.value)])
    finally:
        hb.destroy(buffer)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--font', type=Path, action='append', required=True)
    parser.add_argument('--probe', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    rows = []
    checks = []
    fonts = []
    for path in args.font:
        font = TTFont(path)
        hb = HB(path, 0, font['head'].unitsPerEm)
        names = font.getGlyphOrder()
        fonts.append(dict(path=str(path), sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                          gsub_features=sorted({f.FeatureTag for f in font['GSUB'].table.FeatureList.FeatureRecord})))
        try:
            for text in ['fi', 'fl', 'ff', 'ffi', 'ffl']:
                variants = {}
                cases = [('ltr', text, 4, 'Latn', {}), ('rtl_guess', text, 5, 'guess', {}),
                         ('rtl_latn', text, 5, 'Latn', {}), ('rtl_liga_off', text, 5, 'Latn', {'liga': 0}),
                         ('rtl_liga_on', text, 5, 'Latn', {'liga': 1}),
                         ('rtl_rtlm_off', text, 5, 'Latn', {'rtlm': 0}),
                         ('rtl_rtlm_on', text, 5, 'Latn', {'rtlm': 1}),
                         ('rtl_reversed_input', text[::-1], 5, 'Latn', {}),
                         ('ltr_reversed_input', text[::-1], 4, 'Latn', {}),
                         ('rtl_arab_script_control', text, 5, 'Arab', {})]
                for key, input_text, direction, script, features in cases:
                    result = shape(hb, input_text, direction, script, features)
                    result.update(text=input_text, direction='rtl' if direction == 5 else 'ltr',
                                  requested_script=script, features=features,
                                  glyph_names=[names[g] for g in result['glyph_ids']])
                    variants[key] = result
                ids = lambda key: variants[key]['glyph_ids']
                assertions = dict(guess_is_latn=variants['rtl_guess']['script'] == 'Latn',
                    explicit_latn_unchanged=ids('rtl_guess') == ids('rtl_latn'),
                    liga_on_unchanged=ids('rtl_latn') == ids('rtl_liga_on'),
                    rtlm_off_unchanged=ids('rtl_latn') == ids('rtl_rtlm_off'),
                    rtlm_on_unchanged=ids('rtl_latn') == ids('rtl_rtlm_on'),
                    rtl_equals_ltr_reversed_input=ids('rtl_latn') == ids('ltr_reversed_input'),
                    reversed_input_rtl_equals_ltr=ids('rtl_reversed_input') == ids('ltr'))
                checks.extend(assertions.values())
                hhea = font['hhea']
                core = json.loads(subprocess.check_output([str(args.probe), str(path), '0',
                    str(hhea.ascent-hhea.descent+hhea.lineGap), 'rtl', '-', text], text=True))
                rows.append(dict(font=path.name, text=text, variants=variants, assertions=assertions,
                    fontcore_rtl=core, fontcore_glyph_identity_verified=False))
        finally:
            hb.destroy_font(hb.font)
            font.close()
    output = dict(schema=1, harfbuzz_version=hb.version, fontcore_commit=subprocess.check_output(
        ['git', 'rev-parse', 'HEAD'], text=True).strip(),
        probe_sha256=hashlib.sha256(args.probe.read_bytes()).hexdigest(), fonts=fonts,
        assertion_count=len(checks), assertions_passed=sum(checks), rows=rows)
    args.output.write_text(json.dumps(output, indent=2)+'\n')
    print(f'{len(rows)} cases, {sum(checks)}/{len(checks)} diagnostic assertions')
    if not all(checks):
        raise SystemExit('Unexpected result: inspect JSON; do not treat as parity')


if __name__ == '__main__':
    main()
