# Format 2 contour-point reference: HarfBuzz with FreeType

This is an independent oracle experiment, not a product implementation or a
claim of general Format 2 support. Only installed libraries were used: HarfBuzz
10.2.0 and FreeType 2.13.3. No downloads, Rust builds, fixture changes or product
changes were made.

## Setup and evidence

`tools/compare_gdef_point_carets.py` opens original fixture face 1 with FreeType,
creates the HB font with `hb_ft_font_create_referenced`, sets scale to UPEM
(1000), and sets `FT_LOAD_NO_HINTING | FT_LOAD_NO_BITMAP` (10). It deliberately
does not set NO_SCALE: HB's FT adapter uses scaled 26.6 coordinates internally.
The returned coordinates under this scale are design-unit numerics. The script
records the fixture hash, backend, version, point-callback success, raw design
point, direction-origin-adjusted point, vertical origin and GDEF query.

Known fixture glyph IDs are queried directly. This avoids substituting an
LTR-shaped glyph and pretending it was an actual RTL/TTB text-shaping result.
There is no text-shaping comparison in this experiment.

| Glyph / point | Raw glyf point | FT contour callback | HB Format 2 LTR | HB Format 2 RTL | HB Format 2 TTB |
|---|---|---|---:|---:|---:|
| fi / 1 | (300, 0) | (280, 0) | 280 | 280 | -750 |
| fl / 2 | (850, 500) | (830, 500) | 830 | 830 | -250 |

These are **metric-origin** X coordinates, not the raw outline X: the fixture's
LSB minus xMin is -20. TTB uses vertical origins (450,750) and (550,750), giving
origin-adjusted points (-170,-750) and (280,-250). Its inline y-down distances
are therefore **750 and 250**.

The identical six queries on the HB OT font-functions backend return `[0]`, but
both contour callbacks fail. Those are recorded as
`unsupported_callback_not_a_zero_coordinate`, not as zero-valued authored
carets and not as mismatches against a supported reference.

## Consequences for a future implementation

[HB 10.2 GDEF implementation](https://github.com/harfbuzz/harfbuzz/blob/10.2.0/src/OT/Layout/GDEF/GDEF.hh)
resolves Format 2 through the direction-origin-aware contour callback, unlike
Format 1's scalar coordinate. The measured TTB result already includes the
vertical origin. Negate the query to obtain y-down distance; do not add the
vertical origin a second time. Horizontal RTL query is still an X coordinate;
logical RTL pen distance is advance minus placement minus that coordinate.

[HB FreeType integration](https://github.com/harfbuzz/harfbuzz/blob/10.2.0/src/hb-ft.cc)
provides the callback missing from this OT-only backend. The supported reference
thus validates a bounded unhinted/simple/static path using actual contour point
indices and metric-origin correction. It does not validate interpolated path
vertices as point indices, implied on-curve points, composites, variables,
hinting, invalid indices, or general contour parsing. GPOS placement and actual
text-shaping equivalence were not tested here.

A product implementation can compute from raw glyf points, but must apply the
same drawing metric-origin transform before comparing with this reference.
Returning raw X 300/850 would be inconsistent with the measured unhinted FT
oracle's 280/830. Existing Format 1 normalization must not be blindly reused on
an already origin-adjusted Format 2 HB query.

## Reproduce

```sh
PYTHONDONTWRITEBYTECODE=1 python tools/compare_gdef_point_carets.py \
  --fixture tests/fixtures/gdef-carets.ttc \
  --output doc/gdef-carets/point-carets-reference.json
```

No optional Python FreeType wrapper is required; the script binds installed
FreeType and HarfBuzz C APIs with ctypes. The JSON is diagnostic evidence rather
than a claim that the library under development already supports these cases.
