# Bounded Standard vertical Old Hangul

This stage builds on PR #58, without changing its horizontal behavior or public
APIs. It admits only Standard top-to-bottom, Normal-variant, whole same-face runs
of original exact L V [T] graphemes, each containing an old/extended Jamo.
The 256-glyph limit and transactional/source-ownership guards remain in force.
Mixed upright, sideways, RTL, tones, repeated Jamo, fillers, marks, selectors,
join controls and modern/precomposed-plus-old-T normalization stay excluded.

## Selection and transaction

Keep Standard's existing global `vrt2` preference, otherwise `vert`. Require that
chosen feature to have nonempty lookups in the selected `hang` Script/LangSys.
A feature found only in a foreign Script/LangSys is not borrowed. A font with no
orientation feature also uses legacy shaping. In particular, foreign-only vrt2
plus local vert does not silently switch to local vert.

Jamo and the chosen single orientation feature share one LookupList-order plan
and source-category mask union. A shared lookup runs once. The direct/Extension
Single and bounded ChainContext3 forms, flags, required-feature, variable-feature,
renderability, budget and full rollback constraints are unchanged from #58.

HarfBuzz's liga/clig/rclt defaults are horizontal-only. In the vertical candidate,
each such lookup must be proven not to match at its LookupList position, using a
separate clone and the same dispatch budget. Matching identity substitutions and
matched contextual no-ops also reject the entire candidate. Unsupported/flagged
or shared applicable lookups cannot provide the proof. No probe output is used.
This proves only the candidate's local no-match condition, not that every legacy
or arbitrary reordered run is equivalent to HarfBuzz.

## Positioning and fallback

No new positioning heuristic is added. Existing vmtx/VORG, glyph outlines and
GPOS `vert` SinglePos drive the result. In Noto CJK the V/T Jamo have horizontal
advance zero but vertical advance 1000; authored SinglePos reduces that vertical
advance to zero and positions the components. It must be applied exactly once.
Without those adjustments, nominal vertical advances remain unchanged.

Original source spans/chars remain attached to their glyphs and uncertain
simple-mark ownership is cleared. Family selection still requires both candidate
and legacy coverage, since grouping an unrelated neighbor can select legacy.
Raw scalar routes and no-layout behavior are unchanged. No general Korean,
vertical-mark, BiDi, AAT or browser support is claimed.

## Validation

Original MIT 17-face fixtures cover distinct vert/vrt2 results, ordering,
mask-union/deduplication, foreign Script/LangSys isolation, missing/unsupported
outputs, horizontal-only active/no-match/identity/contextual-no-op/shared cases,
authored positioning versus nominal vmtx, family coverage, and exclusions.
Private unit tests preserve source intervals and raw nominal mapping.

The real-font reference is official OFL Noto Sans CJK KR 2.004, SHA-256
`6bcb2a0703aa137e874fc2dffa85f6c21ba9a67fa329e81b8c801663af7e992a`, compared with
HarfBuzz 15.0.0 OT, Hang/ko, TTB. Size equals hhea ascender minus descender plus
lineGap, normalizing fontcore positions to font design units. Compare final ink,
not raw vertical glyph-origin conventions. Scope/results are recorded with the
final verification; static outline inspection is not GUI/browser testing.

Tone investigation is separate: HB reorders a spacing U+302E/F before a valid
syllable before GSUB, but leaves nominal zero-width tones after it. Orphans and
repeated tones involve dotted circles. No tone change is made by this stage.

References:
- https://github.com/harfbuzz/harfbuzz/blob/15.0.0/src/hb-ot-shaper-hangul.cc
- https://github.com/harfbuzz/harfbuzz/blob/15.0.0/src/hb-ot-shape.cc
- https://learn.microsoft.com/en-us/typography/script-development/hangul
