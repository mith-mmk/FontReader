# RTL glyph-count residual diagnosis

`rtl-residuals.json` records 15 real-font cases, 150 HarfBuzz shaping calls and
15 fontcore probe calls. All 105 diagnostic assertions pass with HarfBuzz 10.2.0.
This is a diagnosis of the 12 RTL glyph-count differences in the actual-direction
comparison, not a new caret or complete shaping-parity claim. Product code is unchanged.

## Result

For all three fonts and all five strings:

- Guessed script is `Latn`; explicitly specifying `Latn` leaves the result unchanged.
  Language is explicitly `en` throughout this experiment.
- Explicit `liga=1` leaves RTL results unchanged. `liga=0` removes the remaining
  `ff` ligature, including the `ff` portion of `ffi`/`ffl`.
- Explicit `rtlm=0` and `rtlm=1` both leave results unchanged. These ASCII letters
  do not demonstrate bracket mirroring, and this is not a mirroring defect.
- RTL(original text) has exactly the same **HarfBuzz glyph IDs** as
  LTR(reversed ASCII input). RTL(reversed input) has the same IDs as LTR(original).
  This experiment reverses ASCII only; it is not a recommended Unicode algorithm.

| Font | LTR fi / fl / ff / ffi / ffl IDs | RTL fi / fl / ff / ffi / ffl IDs |
| --- | --- | --- |
| Noto Sans | 1967 / 1968 / 1966 / 1969 / 1970 | [76,73] / [79,73] / [1966] / [76,1966] / [79,1966] |
| Noto Serif | 2134 / 2135 / 2131 / 2132 / 2133 | [76,73] / [79,73] / [2131] / [76,2131] / [79,2131] |
| DejaVu Sans | 5042 / 5043 / 5041 / 5044 / 5045 | [76,73] / [79,73] / [5041] / [76,5041] / [79,5041] |

IDs 76/79/73 are `i`/`l`/`f` in these exact font versions. Font hashes, clusters,
all feature controls and full core public-probe output are in the JSON. Core
produces one glyph in each of the 15 RTL cases, but its public API does not expose
the glyph ID, so `fontcore_glyph_identity_verified` remains false.

The non-palindromic four strings per font account for all 12 count differences.
`ff` is unchanged by reversal and remains a single ligature in both engines.

## Code evidence and scope

HarfBuzz 10.2.0's `hb_ensure_native_direction` compares requested direction with
script-native horizontal direction. If they differ, it reverses graphemes and
flips the internal direction before substitution. Latin is native LTR, so a
forced RTL Latin run presents `if`, not `fi`, to the Latin ligature lookups.
This is distinct from enabling/disabling `liga`. See the pinned official
[HarfBuzz shaping source, lines 542–595](https://github.com/harfbuzz/harfbuzz/blob/10.2.0/src/hb-ot-shape.cc#L542-L595).

In fontcore `src/fontreader.rs`, `flush_shaped_glyphs` builds `ccmp_glyphs` in
source order (around line 1630); `apply_gsub_sequence_stages` always includes
`rlig/rclt/calt/clig/liga` (1552–1588). RTL adds joining-form processing but not
Latin native-direction reconciliation. Drawing subsequently decreases the pen
(around 3171). Thus it composes logical `fi` first, then places that ligature RTL.
These are different shaping contracts before GDEF is queried; changing GDEF
caret transformation cannot fix the glyph-count discrepancy.

A deliberately incorrect `Arab` script control is also recorded. Noto retains
Latin ligatures there, while DejaVu does not. Script changes also change the
selected GSUB language system and shaper, so this control is explanatory only,
not a workaround or a parity test. Do not label Latin text as Arabic to force a match.

## Narrow follow-up proposal, not implemented

First specify whether explicit `TextDirection::RightToLeft` promises legacy
logical-order ligation with a negative pen, or HarfBuzz-compatible forced RTL
Latin. A compatibility-preserving fix should not silently alter the old API.
For an opt-in script-aware path, reconcile script-native direction **before
GSUB**, retaining original byte/grapheme ownership and restoring the agreed
logical/visual output convention. Start with single-script Latin horizontal runs
and regress `fi/fl/ff/ffi/ffl`, combining marks, source ranges, advances and carets.
Do not reverse all RTL text: Arabic has native RTL shaping, joining, marks and
lookup-direction constraints. Do not reverse raw UTF-8 bytes or scalar strings.

Paragraph base RTL alone does not mean a Latin run should be forced RTL: a proper
BiDi resolver normally assigns Latin an even level. Keep paragraph run resolution
separate from explicit opposite-to-script direction semantics. No production fix
or new public feature-toggle API is necessary for this diagnosis.

## Reproduce

Build `examples/gdef_carets` as described in the main README, then:

```sh
PYTHONDONTWRITEBYTECODE=1 python tools/diagnose_gdef_residuals.py \
  --probe "$CARGO_TARGET_DIR/debug/examples/gdef_carets" \
  --font /usr/share/fonts/truetype/noto/NotoSans-Regular.ttf \
  --font /usr/share/fonts/truetype/noto/NotoSerif-Regular.ttf \
  --font /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf \
  --output doc/gdef-carets/rtl-residuals.json
```

Requires fontTools and installed native HarfBuzz, downloads nothing. The script
imports the existing harness's HB bindings without changing that harness.
