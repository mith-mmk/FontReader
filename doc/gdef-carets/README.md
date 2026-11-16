# GDEF caret reference checks

## Current comparison: actual-direction shaping

`actual-direction-comparison.json` (schema 2) queries GDEF on glyphs that
HarfBuzz actually shapes in the requested direction. It never substitutes an
LTR-selected glyph for an RTL/TTB result. HarfBuzz 10.2.0 uses the OT font
backend, fontTools is 4.61.1, and default fontcore features are enabled.
Font/probe hashes, implementation commit, original source ranges, both advance
axes, positioned ink bounds, raw GDEF records, candidates and final caret
sources are recorded. No font files are copied into the repository.

There are **126 default-feature rows** and **42 supplemental TTB rows**:

| Result | Default | Supplemental TTB `liga=1` |
|---|---:|---:|
| Necessary geometry and candidate coordinates agree; glyph identity unverified | 12 | 10 |
| GDEF carets absent; fontcore approximates | 6 | 5 |
| Shaping/geometry not comparable | 108 | 27 |

These are **not 168 passes**. No comparable row had differing candidate
coordinates, but absent data and incomparable rows are not matches.
The public fontcore API does not expose glyph IDs. Equal source ranges,
advances and positioned ink bounds are necessary evidence, not proof of glyph
identity or complete shaping equivalence. The status deliberately retains
`identity_unverified`. The comparison does not add a production diagnostic API.

### Real fonts

- **LTR:** Noto Sans and Noto Serif `fi/fl/ff/ffi/ffl` all yield a single
  ligature. All 10 rows agree geometrically and in candidate coordinates;
  fontcore adopts `FontAuthored` stops. DejaVu's 5 rows have no corresponding
  GDEF carets and remain `Approximate`.
- **RTL defaults:** Only `ff` yields comparable single-ligature output in each
  font: two Noto agreements and one DejaVu absence. For `fi/fl/ffi/ffl`,
  HarfBuzz and fontcore produce different glyph counts. Those 12 rows are
  **not comparable**, even when an independently selected glyph has similar
  GDEF values.
- **TTB defaults:** All 15 rows are **not comparable**. HarfBuzz produces two
  or three glyphs while fontcore produces a ligature. HarfBuzz's default
  horizontal feature set includes `liga`; the vertical default does not.
  This is a feature-selection/shaping difference, not evidence of a bad GDEF
  caret coordinate.
- **Supplemental TTB with explicit HarfBuzz `liga=1`:** Actual TTB shaping now
  produces the ligature. Ten Noto rows agree in necessary geometry and caret
  coordinates within 0.51 design units; five DejaVu rows still lack GDEF data.
  This **does not replace the failing default-feature comparison**. Noto's
  synthesized vertical origins differ by up to 0.5 units because HarfBuzz
  rounds to integer positions while fontcore retains floats.

All three tested fonts lack `vmtx`. These TTB observations therefore exercise
**synthesized vertical metrics/origins**, not authored vertical metrics or
font-author validation of vertical caret design. There is no pixel, hinted
outline, GUI editing, paragraph BiDi or mixed-orientation conformance claim.

### Synthetic fixtures and why they are not counted as geometry matches

All fixture single-ligature rows have `glyf.xMin=20` but `hmtx.lsb=0`.
HarfBuzz's OT glyph extents account for the bearing difference, while
fontcore's path bounds retain the contour coordinate. For example, face 0
`fi` reports HB bounds `[0,-500,280,0]` versus fontcore
`[20,-500,300,0]`. The 20-unit shift also appears in face 6 with GPOS.
We **do not widen the tolerance, change the fixture, or waive this difference**.
There are 27 default-LTR and 27 supplemental-TTB fixture geometry differences;
the other 54 default RTL/TTB fixture rows differ in glyph count.

The JSON still records raw candidates and final adoption independently:

- Face 0: Format1 `220/730`, with physically reversed LigGlyph blocks.
- Face 1: Format2 point indices retained; fontcore falls back. fontTools lists
  unhinted contour coordinates. HB OT has no contour callback and reports zero,
  which is explicitly **not** used as a valid coordinate reference.
- Face 2: Signed Format3 `-90/-160`. Candidates can agree numerically while
  horizontal layout rejects an out-of-advance position and approximates.
  A negative design coordinate can map inside a vertical advance interval,
  so TTB adoption is recorded independently.
- Face 3: Device data unsupported by fontcore. HB ppem 20 changes horizontal
  `220/730` to `270/830`: +1/+2 pixels become +50/+100 design units. Fontcore
  has no corresponding ppem input. This is not a supported-feature match.
- Faces 4–5: Variable-font candidates conservatively fall back. Valid face 4
  produces HB `245/755` at `wght=650` and `320/830` at `wght=900`; face 5 has
  an invalid VariationIndex. HB returning a base value does not validate it.
- Face 6: Signed GPOS placement is retained in candidate normalization.
- Faces 7–9: Ordered two-caret data can be adopted; reversed and duplicate
  records are rejected, not sorted into valid authored data.

`raw_candidate_diagnostic` is explicitly **not shaping validation** and never
changes an incomparable row into a match. `fontcore_internal_stop_sources`
and `fontcore_authored_adopted` record whether the final layout actually used
font-authored positions rather than merely exposing candidates.

## Coordinate convention

At design-unit scale, with advance `a`, GPOS placement `p`, and raw caret
coordinate `c`, initial-pen distances are `p+c` (LTR) and `a-p-c` (RTL).
For TTB, HB Format1/3 returns the glyph-coordinate Y value without vertical
origin. The screen-down distance is **`-(HB y_offset + c)`**; HB `y_offset`
contains `-vertical_origin` plus placement. Do not compare that directly with
fontcore's `PositionedGlyph.y`, because fontcore also embeds its vertical
origin in path layers. The harness compares positioned bounds instead.
Multiple values reverse for RTL/TTB after conversion; malformed input order
is never repaired by sorting. Format2 is excluded from this normalization
because its contour callback is unsupported by this HB backend.

References: [OpenType GDEF](https://learn.microsoft.com/en-us/typography/opentype/spec/gdef),
[HarfBuzz caret API](https://harfbuzz.github.io/harfbuzz-hb-ot-layout.html#hb-ot-layout-get-ligature-carets),
[10.2.0 GDEF implementation](https://github.com/harfbuzz/harfbuzz/blob/10.2.0/src/OT/Layout/GDEF/GDEF.hh),
[10.2.0 shaping features](https://github.com/harfbuzz/harfbuzz/blob/10.2.0/src/hb-ot-shape.cc).

## Inventory and older snapshot

`installed-inventory.json`, produced by `tools/inventory_gdef_carets.py`,
scans 372 installed font files / 398 faces with zero errors. It finds 3,298
Format1 records, no Format2/3 records, and no GDEF-caret font with `vmtx`.
Thus these local real fonts cannot independently validate Format2/3 or
explicit vertical metrics; synthetic tests remain necessary.

`comparison.json` is the **historical schema-1 raw-query snapshot** from the
previous harness at `e51e294`, based on implementation `54877bc`. Its 34
query agreements were not actual-direction shaping matches. Use schema 2
above for current conclusions. The implementation's separate Rust checks
passed 632 all-feature / 514 default / 297 no-default tests, formatting,
strict clippy, MSRV 1.91 native and stable wasm32 all-feature checks.

## Reproduce

Requires Python with fontTools, native HarfBuzz and the listed local fonts.
The harness downloads nothing. Build externally and move the target directory
to Trash when finished.

```sh
export CARGO_TARGET_DIR=/tmp/fontcore-gdef-target
cargo build --example gdef_carets
PYTHONDONTWRITEBYTECODE=1 python tools/compare_gdef_carets.py --self-test
PYTHONDONTWRITEBYTECODE=1 python tools/compare_gdef_carets.py \
  --probe "$CARGO_TARGET_DIR/debug/examples/gdef_carets" \
  --font /usr/share/fonts/truetype/noto/NotoSans-Regular.ttf \
  --font /usr/share/fonts/truetype/noto/NotoSerif-Regular.ttf \
  --font /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf \
  --fixture tests/fixtures/gdef-carets.ttc \
  --output doc/gdef-carets/actual-direction-comparison.json
PYTHONDONTWRITEBYTECODE=1 python tools/inventory_gdef_carets.py \
  --root /usr/share/fonts \
  --output doc/gdef-carets/installed-inventory.json
```

The tool emits observations and counts, not a blanket CI pass/fail verdict.
`fontcore_size = hhea.ascent - hhea.descent + hhea.lineGap` gives one output unit
per design unit for these faces; it is not a general varied-MVAR comparison.
