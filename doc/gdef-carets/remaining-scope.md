# GDEF follow-up boundaries

## Format 2 implemented boundary

The simple-glyph path accepts unhinted, static, monochrome TrueType real
contour points. The decoder validates its own byte stream, contour endpoints,
instruction length and repeated flags before resolving an index. Actual
on/off-curve points count; implicit curve midpoints and phantom points do not.
Bounded integer-translation composites are also supported as described in
[composite-points.md](composite-points.md). General transformed composites, COLR
and variable cases remain unsupported. At most 16 Format 2 records per glyph are
resolved; larger lists fall back to bound repeated point-decoding work. A separate eligibility
guard checks every accumulated point, contour-adjacent/closing midpoint sum,
baseline calculation and baseline-minus-Y against the legacy renderer's integer
range before labelling the position font-authored. This is not a hardening claim for
the whole pre-existing glyph parser or renderer.

The bounded raw-point decoder and caller are separate because a contour index
is not itself a coordinate. Horizontal points use the metric-origin X shift;
vertical points use Y plus the existing vertical-origin conversion. The
[FreeType-backed oracle](point-carets-reference.md) demonstrates why the old
OT-only backend is not a valid Format 2 reference.

## Device adjustments: low-level decoder only

`FontOptions` currently has nominal size and stretch, but no device ppem, DPI or
hinting context. Nominal size is line-height based, not a pixel-size substitute.
A packed Device adjustment cannot safely be inferred from it. Device carets
therefore remain explicit fallback in mapped shaping. The internal
`CaretValue::device_pixel_delta(ppem)` decoder now handles signed 2/4/8-bit
packed values, rejects zero ppem, and returns zero outside the declared range.
It deliberately does not infer a device context or convert pixels into design
units. A future device-aware context must define axis-specific ppem and scaling
before this decoder can be integrated. No new dependency or public option is
introduced in this work.

## VariationIndex: strict store validation first

`requires/var_store.rs::ItemVariationStore::parse_delta` provides existing delta
calculation, and `fontreader.rs::normalized_variation_coords` supplies fvar/avar
normalization. They are useful foundations, not sufficient fail-closed caret
validation: current tolerant paths can return zero for invalid region indices
or missing axes, and word-count arithmetic needs bounded checking.

GDEF now retains a separately bounded, strict `CaretVariationStore`. Its parser
validates the declared table span, region indices, checked sizes, overlapping
spans and resource budgets; resolution checks outer/inner indices and the
normalized coordinate count. The internal
`Gdef::caret_coordinate` resolver produces finite base-plus-delta values from
explicit normalized coordinates; malformed optional stores do not disable
static carets or legacy shaping.

These are low-level foundations, not a public variable-caret feature. Mapped
shaping still rejects variable faces. Integration must validate fvar/avar
normalization and reconcile varied metrics and rendered geometry before
relaxing that guard. Fixture face 4 at wght 650/900 (+25/+100) and invalid face 5 provide initial regression
controls; they are not a real-font variation coverage claim.

## RTL Latin is a separate shaping contract

The [RTL residual investigation](rtl-residuals.md) identifies native-direction
reconciliation before GSUB, not a GDEF-coordinate defect. Do not reverse all RTL
text or change Arabic joining to conceal it. Any script-aware opt-in design must
preserve grapheme/byte ownership and explicitly state compatibility with the
existing forced-direction API. This work does not change those semantics.
