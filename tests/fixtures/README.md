# Original kana composition fixtures

`kana-composition.ttc` and `generate_kana_composition.py` are original MIT-licensed synthetic test assets. No third-party font outlines are included. Regenerate with fontTools 4.68.0. Fixed timestamps make output reproducible.

The generator lists all 58 two-scalar kana canonical pairs from UnicodeData 17.0.0. Fourteen faces cover both forms, composed-only coverage, original-only coverage, .notdef targets, downstream GSUB and vertical alternates, mixed Hangul/kana contextual rollback, whole-grapheme fallback, and incomplete source coverage. Additional faces exercise context-only damage, a non-first composed ligature input, expansion to missing output, and identity substitution of an existing missing neighbor. Face descriptions are in the generator docstring. Nonzero mark advances intentionally detect accidental generic mark-zeroing. The four Ainu named sequences and all U+31F0–31FF characters are mapped without invented canonical compositions.

# Unsupported variation-selector fixtures

`unsupported-ivs.ttc`, `unsupported-ivs-cff.ttc`, `unsupported-ivs-cff2.ttc` and
`generate_unsupported_ivs.py` are original MIT synthetic fixtures, regenerated
with fontTools 4.68.0. They reuse only the original synthetic vertical-metrics
and CFF/CFF2 vertical-origin fixtures. No vendor font data is included.

TrueType faces: nominal-only; selector record without requested pair; explicit
nondefault; default; explicit glyph zero with contextual GSUB repair; unrepaired
glyph zero; missing combining mark; invalid nominal mark ID; nominal-only face
with distinct base for continuity; valid IVS followed by invalid GSUB mark ID.
CFF/CFF2 collections contain nominal-only, invalid exact base mapping, and exact
base plus invalid exact mark with deliberately malformed COLR metadata. Invalid
IDs are intentional tests, not generally valid fonts.

Rendering ignores unsupported selectors and retains the nominal base. Exact
cmap14 mappings, including explicit zero, are passed to the existing shaping
pipeline unchanged. Family selection first requires whole-cluster shaped glyph
coverage, then prefers exact support for every immediately paired selector.
The existing parser ignores orphan/repeated selectors; a trailing supported
selector never replaces the first pair. Multi-pair EGCs require every pair.
When no usable exact candidate exists, established marked/RTL face continuity
is retained. This does not change Unicode segmentation or internal compressed
source indices; those indices are not original-text scalar offsets after VS.

Compatibility: the crate-private legacy cmap UVS query retains its old result
(including zero for an unlisted pair under an existing selector record). Public
raw glyph/SVG UVS wrappers and layout renderers use the corrected rendering
resolver, so those now retain the base for that unsupported case. No public
signatures change. Explicit glyph-zero mappings remain zero until GSUB repairs
them, if applicable. CFF/CFF2 coverage rejects IDs outside CharStrings before
color fallback because rendering decodes CFF first. No CFF interpreter changes.

Specification references: Unicode 17 chapter 23.4 (unsupported VS invisible and
ignored), OpenType cmap format 14 (supported default/nondefault UVS), CSS Fonts
4 section 5.3 (full variation support before base fallback). This patch does not
claim full CSS fallback or normalization support.

### Kana residual mark advances

`kana-residual.ttc` and `generate_kana_residual.py` are original MIT fixtures,
with fixed timestamps and no external font inputs. Regenerate using fontTools
4.68.0. Faces: 0 no GDEF, 1 GDEF without GlyphClassDef, 2 populated class table
with unclassified mark, 3 authored SinglePos cancellation, 4 zero SinglePos,
5 substituted mark, 6 identity substitution, 7 multiple substitution, 8 ligature,
9 vertical base substitution, 10 PairPos cancellation, 11 zero PairPos,
12 no GPOS, 13 missing mark, 14 authored anchor, 15 expanded base,
16 canonical-composition rollback, 17 missing base, 18 PairPos first-record
cancellation, 19 first-only zero pair, 20 identity locl, 21 deleted base.
Both nominal advances are 1000.
