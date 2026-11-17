# Static bearing correction: real-font regression diagnostic

`bearing-fix-realfonts.json` compares before revision
`d98f6583c44a5946adab50dae786e9e4f4b00d22` with the working-tree bounded
static/simple/monochrome TrueType metric-origin correction. Probe binary and
input font SHA-256 hashes are recorded. Builds used separate disposable Trash
targets; no installed fonts were modified. This diagnostic adds no product code.

## Method

`examples/glyph_origin_probe.rs` accepts arbitrary UTF-8 text and emits public
mapped source ranges, advances, and drawable path commands including positioned
layer offsets. `tools/compare_glyph_origins.py` obtains actual HarfBuzz 10.2.0
`hb_font_draw_glyph` callbacks, not just its extents API. Both paths are evaluated
by fontTools 4.61.1 `BoundsPen`, which computes quadratic/cubic extrema rather
than treating control-point hulls as ink bounds. Both sides use y-down positioned
coordinates and design-unit scale. A 0.51-unit comparison tolerance permits the
known half-unit difference from synthetic vertical-origin integer rounding.
This is unhinted vector geometry, not pixel/raster equivalence. It tests necessary
source/advance/geometry agreement, not proven glyph identity: fontcore's public
mapped API does not expose glyph IDs. Glyph-count, source-cluster or advance
mismatch is classified separately as shaping-not-comparable, never as a pass.

## Results

26 available-font cases: Noto Sans Regular 6, Noto Serif Regular 6, DejaVu Sans 14.
Each available text is checked LTR and default TTB. Texts are A, Å, fi, U+0EB1,
U+20D0, U+2422, and x+U+20D0; unsupported cmap cases are skipped, not passed.

| Classification | Before | After |
|---|---:|---:|
| Necessary geometry agrees, identity unverified | 15 | 23 |
| Positioned path bounds differ | 8 | 0 |
| Shaping not comparable | 3 | 3 |

All 15 previously agreeing rows have identical before/after probe results.
The 8 improved rows are DejaVu Sans's three affected scalars in both directions,
plus x+U+20D0 in both directions. Their horizontal residual changes from -1 to
zero, except the U+0EB1 curved minimum has a remaining approximately 0.0000513
unit numeric difference. The standalone TTB glyphs retain a 0.5-unit Y difference;
this is within the explicit vertical-origin rounding tolerance, not exact
coordinate equality. Advances and mapped source ranges remain unchanged.

The known static simple glyph metrics are:

| Glyph | xMin | LSB | Applied X delta |
|---|---:|---:|---:|
| uni0EB1 | -1186 | -1185 | +1 |
| uni20D0 | -1006 | -1005 | +1 |
| uni2422 | -126 | -125 | +1 |

The three non-comparable rows are `fi` default TTB, one per font: the engines'
ligature feature policies differ. They remain non-comparable after the bearing
fix. This run does not force liga, invent equivalent glyph identities, or count
absence of a comparison as success.

The diagnostic includes Å as an unchanged composite regression sample but does
not prove all composite behavior correct. Variable fonts, COLR, CFF, bitmap and
SVG layers, malformed inputs, hinting, RTL placement and all-glyph coverage are
outside this sample. Product safeguards for excluded branches are tested
separately. See [bearing-residuals.md](bearing-residuals.md) for controlled copies
that establish why the original static xMin/LSB mismatch is valid.

## Reproduce

Build `glyph_origin_probe` once against the before revision and once against the
candidate tree, using separate targets, then run:

```sh
PYTHONDONTWRITEBYTECODE=1 python tools/compare_glyph_origins.py \
  --before /path/to/before/examples/glyph_origin_probe \
  --after /path/to/after/examples/glyph_origin_probe \
  --font /usr/share/fonts/truetype/noto/NotoSans-Regular.ttf \
  --font /usr/share/fonts/truetype/noto/NotoSerif-Regular.ttf \
  --font /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf \
  --output doc/gdef-carets/bearing-fix-realfonts.json
```
