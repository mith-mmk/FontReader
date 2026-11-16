# Bounded Latin canonical composition

Based on draft #60 (which is stacked on #59), independent of the Hangul tone
work in #61. No general normalization or full IPA support is claimed.

## Cause

The three NFD geometry differences recorded by #60 are not arbitrary GPOS
coordinate errors. HarfBuzz chooses font-authored canonical composed glyphs:
`a + tilde + acute` becomes `ã + acute`; `a + ring below + tilde` becomes
`ḁ + tilde`; `a + breve + acute` becomes `ắ`. Noto Sans's composed glyphs and
anchors differ from the outlines/anchors of independently positioned marks.
Changing generic mark-to-mark offsets would therefore corrupt authored data.

## Bounded change

Only horizontal LTR Standard clusters consisting of an ASCII Latin letter and
one or two U+0300–036F marks in nondecreasing, nonzero canonical-class order
are admitted. Data comes from the checked-in Unicode 17.0.0 sources and excludes
compatibility and composition-excluded mappings. The longest renderable exact
canonical prefix is selected, with at most one independent residual mark.
Unsupported composed glyphs fall back to the original scalars; an unsupported
residual mark cancels composition for the entire cluster. GSUB-produced missing
outlines use the existing single rollback to renderable original sources.
Original source spans are retained; canonical composition is not a GSUB ligature.
No new mkmk relaxation or coordinate heuristics are added.

This does not reorder marks, normalize precomposed bases, alter RTL/vertical or
sideways text, or support arbitrary-length clusters. Variation selectors,
joiners, CGJ and marks outside the bounded data are left unchanged.

## Verification

The same Noto Sans Regular 2.004 and HarfBuzz 10.2.0 corpus used by #60 now
matches per-glyph geometry and aggregate ink bounds in all 620 covered rows
(previously 612 and 617). The three differing bounds and five structurally
separate accent cases now match. Noto Sans Hebrew remains 36/36. Reports are in
`doc/latin-composition/`; hashes and comparator tolerances remain recorded.
This is geometry, not pixel, contour, GUI or full BiDi equivalence.

Synthetic font tests cover normal and composed-only faces, missing targets,
missing longest candidate with usable shorter prefix, residual marks, repeated
clusters and cursor advances, GSUB-to-.notdef rollback, and scope rejection.
