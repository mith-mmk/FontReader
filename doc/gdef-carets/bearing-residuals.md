# Static TrueType bearing residual isolation

This is a controlled diagnostic, not a renderer-conformance claim. The product
code and checked-in fixture were not modified. `bearing-residuals.json` records
8 runs of fixture face 0, `fi`, with HarfBuzz 10.2.0 OT font functions and the
existing public `gdef_carets` probe. TTB explicitly enables HB `liga=1`; it is
supplemental, not default TTB shaping equivalence. Temporary font copies live in
Trash. No new Rust build was needed.

## Controlled results

All figures below are LTR unhinted design units. The fixture's outlines are simple
rectangles, so min/max of the draw callback points are exact outline bounds here.
They must not be reused as exact Bézier bounds for arbitrary fonts.

| Variant | glyf xMin | hmtx LSB | HB extents X | HB drawn path X | fontcore positioned bounds X |
|---|---:|---:|---|---|---|
| unchanged face extracted to TTF | 20 | 0 | 0..280 | 0..280 | 20..300 |
| only LSB set to xMin | 20 | 20 | 20..300 | 20..300 | 20..300 |
| only outline translated to xMin=0 | 0 | 0 | 0..280 | 0..280 | 0..280 |
| only LSB set to -40 | 20 | -40 | -40..240 | -40..240 | 20..300 |

TTB produces precisely the same X residual after its horizontal-origin placement:
original HB -450..-170 versus core -430..-150; LSB=xMin gives both -430..-150;
outline-xMin=0 gives both -450..-170; LSB=-40 gives HB -490..-210 versus core
-430..-150. Y bounds and advances agree in these controls.

The residual is `xMin - LSB`, not a scale, GPOS placement, GDEF, or vertical-Y
normalization error. `hb_font_draw_glyph` callbacks independently agree with
`hb_font_get_glyph_extents`: this is not merely an extents API convention.
fontcore bounds come from its drawable path layers; source inspection confirms
raw outline X is retained. No screenshot/raster/hinting comparison was performed.

## Cause and validity

The [OpenType hmtx specification](https://learn.microsoft.com/en-us/typography/opentype/spec/hmtx)
allows static TrueType LSB and xMin to differ. The fixture has head.flags=1,
so bit 1 is clear. This case must not be dismissed as a malformed font. The
same freedom does not apply to the default instance of a variable TrueType font.
JSON records each saved copy's head flags: fontTools recalculates bit 1, yielding
1 for original/LSB=-40 and 3 for LSB=xMin/outline-xMin=0. These flags agree with
the controlled metric relationship; the originals are not edited.

[HB 10.2 Glyph.hh](https://github.com/harfbuzz/harfbuzz/blob/10.2.0/src/OT/glyf/Glyph.hh)
constructs the left phantom point as `xMin - LSB` and translates outline points
by the negative phantom X. Thus drawing is relative to the metric origin.
[HB glyf.hh](https://github.com/harfbuzz/harfbuzz/blob/10.2.0/src/OT/glyf/glyf.hh)
uses this point extraction for paths, separately from its static extents path.

In fontcore, `src/opentype/outline/glyf.rs::Glyph::to_path_commands_parsed`
(lines 559 onward at diagnosis) emits X from outline points plus requested sx,
without LSB correction. `src/fontreader.rs::transform_glyf_commands`
(lines 4374 onward) scales X without translating it. Mapped shaping consumes
these drawable layers. `bearing_x` records LSB separately but does not reposition
the already emitted outline. A consumer additionally translating these paths
would need an explicit documented convention; the current positioned bounds and
HB draw contract do not agree for this valid static case.

## GDEF interaction and recommended scope

All four variants retain GDEF Format 1 coordinate 220. HB raw caret queries
remain 220, core LTR candidates remain 220, and core TTB candidates remain 530.
Changing the outline or LSB does not change the authored GDEF table. Raw candidate
agreement therefore does **not** establish equal ink/caret geometry.

Do not apply `LSB - xMin` to GDEF candidates to hide the residual: HB Format 1
queries are independent of that outline translation. For HB-compatible positioned
rendering, a follow-up correction belongs at the TrueType outline metric-origin
conversion, keeping source bytes, advances, GPOS, and Format 1 caret coordinates
unchanged. This is an existing drawable-origin issue exposed by comparison, not
an error in the new TTB caret Y normalization.

A safe patch should first cover static simple glyphs and explicitly audit
composite USE_MY_METRICS, COLR layer placement, and variable phantom/HVAR paths
before generalizing. Do not shift component outlines independently and then shift
the assembled outline again. Preserve raw-outline APIs if they promise design
coordinates. No product modification was made, and those wider cases were not tested. Until fixed, preserve the geometry mismatch
classification rather than treating matching raw caret numbers as full agreement.

## Reproduce

```sh
PYTHONDONTWRITEBYTECODE=1 python tools/diagnose_gdef_bearings.py \
  --fixture tests/fixtures/gdef-carets.ttc \
  --probe /path/to/built/examples/gdef_carets \
  --scratch /path/to/Trash/gdef-bearing-variants \
  --output doc/gdef-carets/bearing-residuals.json
```

The script only edits newly generated font copies and its specified JSON output.
It captures HB drawing callbacks as well as extents, and does not download fonts.
