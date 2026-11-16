# GDEF caret reference checks

`comparison.json` records an opt-in run with HarfBuzz 10.2.0 and fontTools
4.61.1 against implementation commit `54877bc`. The implementation branch also
passed 632 all-feature, 514 default and 297 no-default tests, strict clippy,
format checks, MSRV 1.91 native and stable wasm32 all-feature checks. These
are separate from the diagnostic rows below. Font files are not copied into the repository. Their filenames and
SHA-256 hashes, face indices, axis settings, reference backend and probe binary
hash are recorded. This checks horizontal caret candidates, not pixel output,
GUI editing, paragraph BiDi, hinted contours or vertical-layout conformance. Real-font TTB versus HarfBuzz
has **not** been compared here; upright/sideways/mixed vertical transformations
are covered separately by synthetic Rust regression tests.

## Result

84 rows: 34 candidate-query matches, 50 absent/unsupported/rejected cases;
no candidate-query mismatches. **Fallback cases are not passes for authored
caret support.** The emitted `stops[].source` distinguishes actual
`FontAuthored` use from `Approximate`.

- Noto Sans and Noto Serif: `fi/fl/ff/ffi/ffl`, LTR, all 10 authored-caret
  cases match HarfBuzz. Their 10 RTL *queries on the LTR-selected glyph* also
  match the initial-pen coordinate conversion.
- That RTL result is **not shaping equivalence**: HarfBuzz shapes Latin
  `fi/fl/ffi/ffl` in RTL into different glyph sequences. Both sequences and
  `same_reference_glyph_selection` are recorded. Raw GDEF lookup tests the
  selected ligature's caret geometry separately from text shaping.
- DejaVu Sans: all 10 rows have no GDEF carets for these ligatures. Fontcore
  explicitly approximates; this is not font-authored data.

The synthetic `gdef-carets.ttc` additionally checks:

| Face | Observation |
|---|---|
| 0 | `fi=220`, `fl=730`; Coverage mapping survives physically reversed LigGlyph blocks. |
| 1 | Format2 point indices are retained but fontcore falls back. fontTools reports unhinted contour coordinates. HarfBuzz's **OT backend has no contour callback** and returns zero here, so zero is not treated as a valid coordinate reference. |
| 2 | Signed Format3 values `-90/-160` agree as candidates; layout rejects them outside the advance interval and approximates. |
| 3 | Device adjustment unsupported by fontcore. HarfBuzz at ppem 20 gives `270/830`, including `+50/+100` design-unit deltas corresponding to `+1/+2` pixels. ppem 0 yields the unadjusted `220/730`. Fontcore has no matching ppem parameter. |
| 4 | Variable-font carets conservatively fall back. HarfBuzz with `wght=650` gives `245/755` (avar maps normalized .5 to .25), and `wght=900` gives `320/830`. |
| 5 | Invalid VariationIndex still falls back. HarfBuzz's base-coordinate result does not prove the index valid. |
| 6 | Signed GPOS placement plus carets match: LTR `270/700`, RTL initial-pen distances `630/400`. |
| 7 | Two ordered `ffi` carets match; RTL logical distances reverse appropriately. |
| 8–9 | Reversed or duplicate font records are rejected rather than silently sorted into valid authored carets. |

## Coordinates and limitations

HarfBuzz's GDEF API returns scaled glyph-origin coordinates. For a horizontal
ligature with advance `a`, placement `p`, and GDEF coordinate `c`, fontcore's
logical initial-pen distance is `p+c` for LTR and `a-p-c` for RTL. The RTL
screen point is the negative distance from the run's initial right edge. The
harness uses HarfBuzz's LTR-selected glyph advance/placement as an independent
reference and compares candidates within 0.51 design units (integer rounding).
It preserves the original HarfBuzz record order in JSON even when displaying
logical RTL distances sorted for comparison. Fontcore's own rejection of
invalid order is checked separately by the fixture.

Fontcore's public result does not expose glyph IDs. The diagnostic pairs a
single fontcore glyph with the single HarfBuzz LTR glyph only when their
advances agree, records that reference basis, and does not claim general
identity or shaping equivalence from this pairing. `candidate_query_match`
also does not mean the final caret layout accepted the candidate.

Reference semantics:
[OpenType GDEF](https://learn.microsoft.com/en-us/typography/opentype/spec/gdef),
[HarfBuzz caret API](https://harfbuzz.github.io/harfbuzz-hb-ot-layout.html#hb-ot-layout-get-ligature-carets),
and the tested version's
[GDEF implementation](https://github.com/harfbuzz/harfbuzz/blob/10.2.0/src/OT/Layout/GDEF/GDEF.hh).

## Reproduce

Requires Python with fontTools, the native HarfBuzz shared library and the
listed local font files. No network downloads are performed by the harness.
Use an external disposable build directory and move it to Trash when finished.

```sh
export CARGO_TARGET_DIR=/tmp/fontcore-gdef-target
cargo build --example gdef_carets
PYTHONDONTWRITEBYTECODE=1 python tools/compare_gdef_carets.py \
  --probe "$CARGO_TARGET_DIR/debug/examples/gdef_carets" \
  --font /usr/share/fonts/truetype/noto/NotoSans-Regular.ttf \
  --font /usr/share/fonts/truetype/noto/NotoSerif-Regular.ttf \
  --font /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf \
  --fixture tests/fixtures/gdef-carets.ttc \
  --output doc/gdef-carets/comparison.json
```

The tool emits observations and summary counts, not a CI pass/fail verdict.
`fontcore_size = hhea.ascent - hhea.descent + hhea.lineGap` makes these runs use
one output unit per font design unit. This setup is not a generic varied-MVAR
or rasterizer comparison.

## Rust regression verification

The implementation checkpoint passed 632 tests with all features, 514 with
normal defaults, and 297 with no default features (including doctests).
`cargo fmt --check`, all-target/all-feature Clippy with warnings denied,
Rust 1.91 all-feature native check, and stable all-feature `wasm32-unknown-unknown`
check also passed. Self-contained tests cover malformed caret offsets/counts,
physical-versus-Coverage order, signed coordinates, multi-caret ordering,
GPOS placement, scale/stretch, LTR/RTL/TTB, explicit fallback, and propagation
through family, composite, hard-line, BiDi and mixed-vertical adapters.
This is a bounded extension, not complete GDEF support or a general editor model.
