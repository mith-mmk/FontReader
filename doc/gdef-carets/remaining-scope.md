# GDEF follow-up boundaries

## Format 2 implemented boundary

Only unhinted, static, simple, monochrome TrueType real contour points are
eligible. The decoder validates its own byte stream, contour endpoints,
instruction length and repeated flags before resolving an index. Actual
on/off-curve points count; implicit curve midpoints and phantom points do not.
Composite, COLR and variable cases remain unsupported. At most 16 Format 2 records per glyph are
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

## Device adjustments: deferred, not guessed

`FontOptions` currently has nominal size and stretch, but no device ppem, DPI or
hinting context. Nominal size is line-height based, not a pixel-size substitute.
A packed Device adjustment cannot safely be inferred from it. Device carets
therefore remain explicit fallback. A future device-aware context must define
axis-specific ppem and scaling before a resolver can be added. No new dependency
or public option is introduced in this work.

## VariationIndex: strict store validation first

`requires/var_store.rs::ItemVariationStore::parse_delta` provides existing delta
calculation, and `fontreader.rs::normalized_variation_coords` supplies fvar/avar
normalization. They are useful foundations, not sufficient fail-closed caret
validation: current tolerant paths can return zero for invalid region indices
or missing axes, and word-count arithmetic needs bounded checking.

GDEF currently retains the ItemVariationStore offset. A follow-up must bound the
store to the declared GDEF table, validate outer/inner and region indices, axis
counts and all size arithmetic, then calculate finite base-plus-delta values.
Only proven supported cases should relax the variable-face fallback. Fixture
face 4 at wght 650/900 (+25/+100) and invalid face 5 provide initial regression
controls; they are not a real-font variation coverage claim.

## RTL Latin is a separate shaping contract

The [RTL residual investigation](rtl-residuals.md) identifies native-direction
reconciliation before GSUB, not a GDEF-coordinate defect. Do not reverse all RTL
text or change Arabic joining to conceal it. Any script-aware opt-in design must
preserve grapheme/byte ownership and explicitly state compatibility with the
existing forced-direction API. This work does not change those semantics.
