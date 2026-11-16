# Bounded Old Hangul tone placement

Builds on PR #59. This change handles exactly one U+302E or U+302F following a
strict old L V [T] grapheme already supported by the bounded Hangul shaper.
Spacing tones (nominal horizontal advance nonzero) move before their syllable
**before GSUB**; zero-width tones remain after it. Tone glyphs receive global
features, including the selected vertical feature, but not ljmo/vjmo/tjmo.
Original source intervals survive visual reordering. Mark ownership remains
cleared, and font-family coverage still requires both candidate and legacy
outputs because a later unsupported neighbor can trigger whole-run fallback.

The candidate is transactional: any unproven input or lookup restores the whole
legacy run. Variable fonts carrying HVAR, fvar, or gvar are excluded for tones,
because this stage does not receive the active variation coordinates needed to
classify tone advance reliably. No variation behavior is changed for unmarked
Old Hangul.

## Scope limits

No modern NFD-with-tone composition, precomposed syllable + tone handling,
orphan/repeated tones, dotted-circle insertion, repeated Jamo, mixed-script
runs, BiDi, sideways or mixed vertical shaping is added here. Those remain on
the existing route. This is not complete Hangul tone support. No GUI/browser
or native rendering comparison has been performed.

## Reference verification

Official Noto Sans CJK KR Regular 2.004, SHA-256
`6bcb2a0703aa137e874fc2dffa85f6c21ba9a67fa329e81b8c801663af7e992a`, was compared
with HarfBuzz 15.0.0, script hang, language ko. At fontcore size 1448 (hhea
ascent minus descent plus lineGap), fontcore final ink bounds and advances
match HarfBuzz in all 28 cases: seven strict old syllables, both tones, LTR and
TTB. Comparing final ink accommodates their different vertical-origin
conventions. The results include every glyph, not only total advances.

Results: [machine-readable comparison](hangul-tone/noto-reference.json).
The supplied font is not committed. To reproduce with the original font:

```sh
cargo build --example hangul_tone_probe
python tools/compare_hangul_tones.py /path/NotoSansCJKkr-Regular.otf target/debug/examples/hangul_tone_probe
```

The Python comparator requires fontTools and uharfbuzz, reads the input font
without modification, and exits unsuccessfully on any inequality. Neither it
nor the normal test suite downloads fonts. A match on this small pinned corpus
does not claim all-font coverage.

Original MIT synthetic fixtures cover spacing and zero-width order, global
substitution versus category masks, vertical substitution and authored GPOS,
variable-font fallback, source ownership and font-family fallback. Existing
exclusion controls now use repeated tones, which remain unsupported.

References:
- https://github.com/harfbuzz/harfbuzz/blob/15.0.0/src/hb-ot-shaper-hangul.cc
- https://learn.microsoft.com/en-us/typography/script-development/hangul
- https://github.com/notofonts/noto-cjk/tree/main/Sans/OTF/Korean

## Local regression validation

Default tests: 436 passed. `--no-default-features`: 246 passed.
`--all-features`: 554 passed. Formatting and all-target/all-feature Clippy
with warnings denied passed. These are automated checks, not GUI validation.
