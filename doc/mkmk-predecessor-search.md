# Bounded lookup-specific MarkToMark predecessors

Ordinary and extension Mkmk lookups now select their own predecessor in a
conservative, provably simple mark run. Current-input GDEF filtering is
unchanged. The predecessor scan retains MarkFilteringSet/MarkAttachmentType
filtering but does not use IgnoreBaseGlyphs, IgnoreLigatures or IgnoreMarks
as permissions to cross other glyph classes. Set filtering precedes attachment
classes. The first admitted target ends the scan even when its coverage or
anchor is missing. A later lookup may retry with its own properties. Zero
anchor deltas are successful, and the existing first-success lookup order and
Y-up to Y-down conversion remain unchanged.

## Ownership bound

A private optional original-grapheme ID is retained only by nominal one-to-one
inputs. All GSUB substitutions, including identity substitutions, clear it;
so do composition, composition restoration and Tibetan normalized output.
Multiple outputs cannot inherit simple ownership. Selector-consuming inputs and graphemes containing ZWJ/ZWNJ
are excluded. This is provenance, not a claim that source intervals alone
encode attachment ownership.

The ownership proof is evaluated lazily only when a lookup encounters an
excluded target, and cached across lookups for that current glyph. Successful
adjacent attachments do not acquire a full-prefix scan. Filtered searches and
existing fallback can still have quadratic worst-case cost on long mark runs.

New target filtering/skipping requires the complete span from an explicit
GDEF Base through the current Mark to have one common original grapheme ID,
only GDEF Marks after that Base, one component per glyph, consecutive width-one
source intervals, and already emitted predecessor glyphs. It stays within the
renderer-provided face/run/orientation slice and cannot cross tabs/newlines.
Leading marks, unclassified owners, ligatures, transformed/composed sources,
and ambiguous ownership do not qualify.

When this guard fails, the old immediate-target selection and target-filter
behavior is retained. Missing GDEF, missing GlyphClassDef and unclassified
adjacent targets remain coverage-driven. No Unicode-derived GDEF class is
invented. These cases are compatibility controls, not HarfBuzz parity claims.

Separately, an explicitly classified GDEF Base, Ligature or Component is now
rejected as a Mark2 target, even if Mark2Coverage contains it. This tested
correction also applies to adjacent targets. Unknown classes are unaffected.

## Deliberate limits

This does not implement per-mark ligature IDs/components, unrestricted mark
ownership, canonical reordering, a full sequential GPOS executor, or new
MarkToBase/MarkToLigature target search. First-success behavior is retained
rather than claiming every later successful lookup is applied. Guarded-out
runs may therefore still differ from HarfBuzz. Missing/out-of-range filtering
sets retain the existing rejection policy, which differs from HB15. The
unanchored-mark policy from the preceding change is unchanged.

Original MIT fixtures in `tests/fixtures/mkmk-predecessor.ttc` use equal-CCC
marks to isolate predecessor selection from reordering. Tests include ordinary
and extension lookups, selected origins, zero anchors, lookup fallback/order,
filtering precedence, explicit nonmark targets, missing data, substitutions,
run barriers and no-layout output. Private unit tests exercise ownership
metadata, interval mutations, missing rendered targets, compositions,
restoration and Tibetan normalization.

## Primary references

- HarfBuzz15 target scan, nonmark stop and ligature ownership checks:
  https://github.com/harfbuzz/harfbuzz/blob/15.0.0/src/OT/Layout/GPOS/MarkMarkPosFormat1.hh
- OpenType lookup flags:
  https://learn.microsoft.com/en-us/typography/opentype/spec/chapter2#lookupFlags
- OpenType GPOS lookup/attachment semantics:
  https://learn.microsoft.com/en-us/typography/opentype/spec/gpos

## Bounded real-font controls

Independent read-in-place Arial5.01.2x comparisons found two unchanged-glyph-order
cases: `q + U+0301 + U+FE20 + U+0301` and
`q + U+0323 + U+0301 + U+0345`. At UPEM2048 the final marks now match HB15
at (814,-25) and (572,721), respectively, in Y-down coordinates. The preceding
version produced (825,328) and (575,424). Other glyph positions also match.
These examples establish applicability without relying on canonical reordering;
they are not a general shaping-parity claim. Installed fonts were not copied.
