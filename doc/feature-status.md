# Feature Status

## Opt-in layout and bounded GDEF carets

New source-preserving and upper-layout APIs are described in [mapped shaping](mapped-shaping.md), [hard lines](hard-line-layout.md), and [bounded BiDi/mixed vertical](bidi-and-mixed-vertical.md). Legacy shaping limits in the sections below remain in force unless those opt-in routes explicitly say otherwise. Source ranges are conservative grapheme envelopes. Static GDEF Format 1, bounded simple or integer-translation composite TrueType Format 2, and unadjusted Format 3 carets are used when validated; unsupported cases retain explicitly labelled approximation. Upper layouts do not provide family fallback, wrapping or cross-run joining. This is not general Unicode/script conformance.


This document keeps the implementation-oriented notes that were previously in `README.md`.
The README now focuses on the public API and runnable examples.

## Variation-selector rendering and fallback

Unsupported UVS/IVS render the nominal base with the selector ignored, whether
the font lacks format 14, the selector record, or that base/selector pair.
Exact default/nondefault mappings are retained; explicit glyph zero reaches
contextual GSUB unchanged. This corrects the semantics of raw UVS glyph/SVG
wrappers as well as layout rendering, without changing their signatures. The
crate-private legacy cmap lookup keeps its historical return behavior.

Family selection requires every shaped glyph in the EGC to be renderable
before preferring exact support for all immediately paired selectors. It
rejects invalid CFF/CFF2 CharString IDs even when color metadata exists. Existing
marked/RTL continuity stays intact when no usable exact alternative exists.
Only the first immediately attached selector is used; trailing/orphan selectors
follow the existing parser policy. Original EGC text is retained, but internal
GSUB indices omit selectors and are not original scalar offsets.

No Unicode registered-sequence filter is added: a font-authored unregistered
mapping can be honored here even where Chrome ignores it. This is not full CSS
fallback or universal browser parity. See [fixture coverage](testing.md#variation-selector-regressions).

## Unicode vertical-orientation property

`vertical_orientation(char)` exposes Unicode 18.0.0 `Vertical_Orientation` as
four distinct values (`U`, `R`, `Tu`, `Tr`), without fonts or feature flags.
`Tr` prefers a transformed upright form and rotates only as a fallback.
The scalar query does not resolve grapheme clusters or font substitutions and
does not change existing shaping. Mixed orientation is available only through
the explicit [mixed vertical LTR outline APIs](#opt-in-mixed-vertical-ltr-outlines),
not through the scalar query or the ordinary shaping policy.
See the [recipe](api-recipes.md#query-unicode-vertical-orientation) and
[data provenance](../data/unicode/README.md).

## Layout support

`layout` is partially implemented.

### Bounded canonical kana composition

With `layout`, exactly 58 Unicode 17.0.0 canonical hiragana/katakana pairs
(base plus U+3099 dakuten or U+309A handakuten), including voiced iteration
marks, compose before cmap only when the entire extended grapheme is that pair
and the selected face can render the composed glyph. Source characters and
scalar spans remain original. Composed-only faces work; absent/invalid targets
retain the original cluster and whole-grapheme fallback. CFF/CFF2 CharStrings
bounds are checked. Hangul and kana share a single bounded restoration pass if
downstream GSUB produces unusable output, preserving composed-only neighbors.
Existing horizontal, vertical and mixed-orientation GSUB semantics still apply.
Raw scalar APIs and builds without `layout` do not compose.

Restoration requires nominal cmap coverage for every original scalar. Custom
fonts that repair missing original cmap entries through GSUB are outside this
guarantee. This does not repair generic contextual FontFamily fallback: a
composed-only cluster that later becomes missing keeps the existing downstream
missing-glyph policy, like its NFC equivalent. No public cluster/caret API,
generic GSUB ligature source-span repair, or change to existing selector
omission in internal run indices is added. Untouched pre-existing missing
neighbors no longer trigger restoration. An internal substitution flag retains
the conservative whole-run guard for any unusable substituted output, including
identity substitutions, expansions, ligatures and contextual substitutions. A
composition can influence another glyph as context, so this is deliberately not
exact causal localization. Such failures still restore every restorable kana or
modern Hangul composition once, without splitting the GSUB run.

This is not general NFC/NFKC, decomposition, reordering, Latin normalization,
halfwidth conversion, Old Hangul or complete Ainu shaping. Extra marks,
selectors, joiners and prepends exclude the whole grapheme from this stage.
The four Ainu named sequences セ+309A, ツ+309A, ト+309A and ㇷ+309A have no
canonical composed scalar and remain unchanged, as do U+31F0–31FF. Unligated
marks are not assigned generic zero advance. Font-specific Ainu mark placement
and broader script shaping remain unresolved; language and script are distinct.

### Bounded modern Hangul composition

With `layout`, text shaping can compose a complete Unicode 17 extended
grapheme containing exactly modern L+V, L+V+T, or precomposed LV+T. The target
is selected before nominal cmap mapping and only when this face contains a
nonzero, renderable syllable glyph. A face may therefore support NFD even
without its jamo cmap entries. Missing syllables preserve the complete original
grapheme; a supported LV prefix never replaces part of an unsupported LVT.

This is font-aware modern canonical composition, **not Old Hangul or general
Korean shaping support**. Old/extended jamo, fillers, repeated jamo, compatibility
jamo, tones, selectors, join controls, Prepend characters and unrelated marks
exclude their entire grapheme. Adjacent Latin and other graphemes do not exclude
a valid modern syllable. No general NFC/NFKC conversion, Hangul decomposition,
`ljmo/vjmo/tjmo` masks, tone reordering, dotted-circle insertion, locale mapping,
line-breaking or browser parity is added. Existing downstream GSUB selection
and orientation stages remain in use.

The composed glyph retains the first original source character and ownership
of its complete admitted grapheme. Unicode source count is separate from GSUB
ligature components: composition starts as one nominal glyph. Legacy
source-bearing exports retain the original character. This does not add a
public cluster/caret API or repair generic GSUB ligature source spans. Existing
selector omission in internal run indices is unchanged.

FontFamily probes and rendering share this path and retain the original
grapheme as the fallback unit. If a newly composed run produces an unrenderable
GSUB output, it is retried once with every restorable composition replaced by
its fully renderable original jamo. Composed-only neighbors stay composed.
An untouched pre-existing missing glyph neither triggers nor prevents that retry. This is
a bounded protection for newly composed inputs, not generic contextual fallback:
a composed-only syllable that becomes missing under later context retains the
existing downstream missing-glyph policy, as its NFC equivalent does. Restoration
requires nominal cmap coverage for every original scalar; custom fonts that
repair missing original cmap entries through GSUB are not covered by this
guarantee.

Raw scalar APIs and layout-disabled shaping stay unchanged. The algorithm
follows [Unicode 17 section 3.12](https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-3/#G24646).
See [tests and limitations](testing.md#bounded-modern-hangul-composition-0018).

### Bounded Old Hangul Jamo

A separate conservative horizontal LTR candidate supports exact archaic/extended
L V [T] graphemes using masked jmo features. See [scope and limitations](bounded-old-hangul.md).
This does not change the modern-composition scope above or provide general Korean shaping.

### Basic Tibetan vowel anchors

With `layout`, a bounded GPOS fallback handles an adjacent, unligated Tibetan
consonant (U+0F40–U+0F6C, excluding unassigned U+0F48) and simple vowel
(U+0F71, U+0F72, U+0F74, U+0F7A–U+0F7D, or U+0F80). Existing `mkmk` and
`mark` attachment retain priority. Only when they find no attachment does the
fallback search `abvm`/`blwm` MarkToBase lookups in the explicit `tibt` script.
It retains locale language-system selection and does not fall through to
`DFLT` or unrelated scripts. GDEF lookup filtering is honored; flagged lookups
are skipped when the needed GDEF information is unavailable. Anchor deltas use
the normal screen-coordinate conversion and font/stretch scale.

### Bounded Tibetan substitution runs

With `layout`, a separate `tibt` plan can form single-glyph stacks and select
simple-vowel alternates. After the bounded canonical preprocessing described
below, every original grapheme must contain a Tibetan base, optional subjoined
consonants, and at most one simple vowel or the lower-vowel tail AA+U. A stacked
consonant followed by exactly one upper vowel has the additional proof below. At least one base needs
a component. Spaces, punctuation, another script, unsupported vowels, variation
selectors and joiners disable the plan for the entire run. Tabs/newlines retain
their existing run boundaries. This restriction applies to horizontal and
explicit vertical routes; it is not general script itemization or complete
USE/Tibetan support.

The font must expose `abvs` or `blws` in the selected `tibt` language system.
`locl`/`ccmp` run as cluster preprocessing; `abvs`, `blws`, existing contextual,
ligature and variant features run together across the **entire accepted run**
in LookupList order, deduplicated within each stage. Existing orientation
selection remains separate from script selection. The plan keeps locale
language selection without adding a global `bo`/`dz` script mapping or falling
through to unrelated scripts.

The candidate is transactional. It must contain an effective `abvs`/`blws`
change, consume every below vowel/subjoined source into a base-owned ligature,
and preserve exact contiguous component ownership within each original
cluster. Executed expansion/deletion and cross-cluster ligatures are rejected.
Failure restarts the unchanged legacy path from original cmap glyphs, including
its scalar `locl`. Generic GPOS ligature-component selection remains unchanged.
Family support probing checks both candidate and legacy glyph coverage because
later same-face grouping can disable the plan.

The default bounded path executes flagged GSUB lookups only when GDEF proves
**every glyph currently in the run is included**. It reads mark-filtering sets,
respects IgnoreMarks/filter-set/attachment-class precedence, rejects missing
required data, and invalidates cached proofs when substitutions change present IDs.

A run containing a newly admitted stack plus one upper vowel explicitly enables
the [bounded filtered-context exception](../docs/filtered-context.md). When the
whole-run proof is unavailable, this permits Single and contiguous Ligature
substitution, and chaining context format 3 with contiguous included input and
zero or one nested record, including supported extension wrappers. Backtrack and
lookahead may skip excluded glyphs; consumption may not skip an interior mark.
Nested lookups use their own flags and remain inside ancestor input bounds.
Unsupported matches, source ambiguity or exhausted execution/work limits restore
the entire candidate, rather than returning a partially shaped run.

This opt-in applies to the entire presentation transaction, including other
admitted plain/simple-vowel/lower-stack neighbors in that run. It does not add
script itemization: unsupported vowels, punctuation and mixed-script/spaced runs
still disable the bounded plan. Runs without a new upper stack retain the default
path. Individually supported stacks can still roll back in combination or when
a limit is exhausted. No full-font or full-script parity is claimed.

#### Bounded stack plus one upper vowel

The newly admitted normalized grammar is base + one or more subjoined consonants
+ exactly one of U+0F72/U+0F7A–U+0F7D/U+0F80. After all selected substitutions,
each such cluster must produce exactly one explicit GDEF **Base** owning every
consonant, followed by one explicit GDEF **Mark** owning the upper vowel. Exact
contiguous original ownership is mandatory. Canonical consonant components may
share one Base output; no original scalar may be split across outputs.

A real selected `tibt` `abvm` MarkBasePos anchor (type 4 or extension-to-type-4)
is mandatory before committing the run. Its unscaled adjustment is retained
privately for positioning. Ordinary `mark` Base anchors retain priority; no
type-5 component is guessed from the number of consumed consonants. Missing
GDEF, unknown/wrong classes, absent anchors, filtered participants, missing
outputs, or an unresolved subjoined glyph restore the complete legacy run.
Validation walks accepted clusters linearly rather than rescanning the output.

This does not add AA+upper, compound U+0F73/U+0F81, mixed upper/lower sequences,
multiple upper vowels, punctuation itemization, generic transformed-mark Mkmk,
or a ccmp-only activation path. Runs without a newly admitted upper stack retain
their existing lower-only and simple-vowel behavior. Run context can still disable
individually safe stacks.

#### Bounded canonical normalization

Canonical Tibetan properties are pinned to **Unicode 17.0.0**, matching the
original grapheme boundaries, independently of the Unicode 18 property tables.
The 17 canonical decompositions and 19 nonzero combining classes come from the
checked-in official UnicodeData.txt, with a checksum-verifying offline generator
and independent Rust source parser. There is no host-Python Unicode dependency.

Only a private candidate buffer is decomposed, and only when both components
have renderable cmap coverage in the current face. Missing components preserve
the original scalar. Stable canonical ordering moves complete source records
and stops at CCC 0 (including subjoined consonants). More than two nonzero-CCC
marks are rejected before sorting. There is no recomposition, compatibility
mapping, HarfBuzz-modified CCC, or general-purpose NFD API.

The accepted new forms are decomposable base/subjoined consonants and U+0F75
(UU), including literal AA+U and reordered U+AA, **only if the font's selected
lookups fully collapse all lower/subjoined components**. Some fonts still fall
back for compound subjoined consonants. U+0F73/U+0F76/U+0F78/U+0F81, AA+above
vowels, multiple/mixed upper vowels, U+0F77/U+0F79 compatibility forms, orphan dotted
circles and punctuation/mixed runs remain unsupported by this plan.

Each candidate keeps the normalized shaping scalar separate from the original
source character, original grapheme, and scalar interval. Decomposition copies
ownership; reordering moves it; ligation must cover an exact contiguous original
interval without sharing a decomposed source between outputs. GSUB ligature
component counts still count normalized glyph components, not original scalars.
Source-bearing legacy exports retain original characters. No public structs or
source-mapping API are added. New normalized/AA+U/upper-stack candidates also roll back if
the final output lacks coverage, preserving available originals. Unchanged
previously admitted runs keep their prior missing-output fallback policy.

FontFamily still selects using original graphemes and requires both candidate
and legacy coverage, since later grouping can disable the plan. Thus an explicit
single face can normalize a missing compound from available components while
FontFamily conservatively rejects that face if the original legacy path is
incomplete. Raw single-glyph APIs and layout-disabled behavior are unchanged.
See [Unicode data provenance](../data/unicode/README.md#tibetan-canonical-properties-unicode-1700),
[USE feature stages](https://learn.microsoft.com/en-us/typography/script-development/use#opentype-feature-application-ii),
and [UAX #15 revision 57](https://www.unicode.org/reports/tr15/tr15-57.html).

### Vertical metrics fallback

`FontEngine::shape`, measurement, and SVG keep top-to-bottom flow even when a
face lacks the vhea/vmtx pair. In that case the vertical advance is synthesized
from the horizontal ascender minus descender (units per em for nonpositive
extents), scaled once with the existing font-size scale. Horizontal stretch does
not change this advance. A vhea header without vmtx also takes this fallback.
Zero horizontal advances stay zero in the fallback, including unattached marks
and zero-width characters. Existing vmtx advances, including zero, are retained
when both tables exist.
Without the `layout` feature the sfnt loader does not read these tables, so it
uses the same fallback.

The committed synthetic fixtures exercise fonts with and without vertical
metrics and mixed collection faces without requiring an installed font. This
fallback is distinct from the TrueType origin correction below. Unicode
orientation is not inferred by this legacy flow; opt-in mixed layout is described
[below](#opt-in-mixed-vertical-ltr-outlines). Static CFF origins are covered below.

### TrueType variable horizontal metrics

Horizontal phantom points start from the selected face's default `hmtx`.
`HVAR` and `gvar` are alternative metric sources: available HVAR advance/LSB
entries take precedence without adding both deltas. A missing optional LSB map
keeps the gvar-derived bearing; a mapped zero remains authoritative. HVAR still
applies when no gvar tuple matches. This works without the `layout` feature.

The [HVAR specification](https://learn.microsoft.com/en-us/typography/opentype/spec/hvar)
describes defaults and optional mappings. Conflicting-table fixtures test our
source-selection policy, not conforming font authoring or a required conflict
resolution rule. MIT fixtures cover implicit/explicit maps, TTC faces, sparse
phantom tuples, signed/fractional deltas, scale/stretch, measure/SVG bounds and
saturation diagnostics. Saturation cases test robustness outside valid metrics.

Composite-glyph tuples use component indices followed by four phantom points,
not flattened outline indices. Component XY deltas are applied before offset
scaling; omitted component deltas remain zero rather than using contour IUP.
Default phantoms use the original composite bounds and the selected face's
`hmtx`/`vmtx`; varied outline bounds determine bearings, including when only a
child has an applicable tuple. HVAR/VVAR precedence remains per field. Nested component offsets retain
fractional precision until the outer outline is converted to `ParsedGlyph`;
`ROUND_XY_TO_GRID` is not applied as design-unit rounding in this unhinted path.
The public integer outline representation and simple-glyph rounding are
unchanged, so this is not a general sub-font-unit precision guarantee.

The dedicated MIT fixture and independent fontTools 4.68.0 instancing oracle
cover translated and nested XY components, uniformly scaled component offsets,
sparse tuples, signed/fractional phantom deltas, child-only variations, selected
TTC faces, raw getters, shaping, scale/stretch, measurement and SVG bounds.
Nonzero-left-phantom and conflicting-table cases are diagnostic, not conforming
variable-font authoring examples. These are composite glyphs within a face,
distinct from multi-font fallback.
See [composite gvar processing](https://learn.microsoft.com/en-us/typography/opentype/spec/gvar#point-numbers-and-processing-for-composite-glyphs).

Limits: point-attached composites still use the legacy unvaried outline path;
correct point matching is not implemented. `USE_MY_METRICS` inherited phantom
positions/hinting are not implemented: its own phantom deltas are ignored and
metrics retain the `hmtx`/HVAR and `vmtx`/VVAR fallback while supported XY outlines
vary. Fallback tests verify these boundaries, not conformance. Arbitrary rotated
or sheared composites are not covered by this regression suite.

### TrueType variable vertical metrics

Vertical phantom points start from default `vmtx` metrics. `gvar` deltas and
`VVAR` deltas are alternative sources, so matching adjustments are applied once.
Each available `VVAR` advance or top-side-bearing entry takes precedence over
the corresponding derived `gvar` metric; an omitted optional TSB mapping keeps
the `gvar` result, including changes caused by varied outline bounds. Explicit
zero deltas still take precedence. TrueType ignores `VVAR` vertical-origin maps.

The [VVAR specification](https://learn.microsoft.com/en-us/typography/opentype/spec/vvar)
describes the alternate metric access path and optional side-bearing mappings.
Conflicting tables are tested as a deterministic source-selection policy, not
as conforming font authoring or a spec-mandated conflict-resolution rule.
Committed MIT fixtures cover default/middle/maximum axis values, optional
mappings, selected collection faces, shaping, measurement, and ink origins.
These vertical fixtures are separate from the horizontal coverage above.

### TrueType vertical origins

Monochrome TrueType layers use half the horizontal advance as the x origin and
`glyf.yMax + vmtx.topSideBearing` as the y origin, per the
[OpenType vmtx specification](https://learn.microsoft.com/en-us/typography/opentype/spec/vmtx).
Without vertical metrics, outlines are centered vertically in the synthesized
cell. Glyph-local layer offsets are scaled once; logical column positions and
advances stay unchanged. Bounds and SVG use the same translated layers. Marks
attached by GPOS/GDEF inherit the base glyph's origin translation.

Synthetic regressions cover punctuation, negative bearings, composites, gvar
outline/phantom movement, collection face selection, and sfnt/WOFF parity.
BASE alignment and color origins outside the static COLR v0 case below remain
separate work. Orientation is supplied by the explicit mixed API, not inferred
by this legacy origin correction. The legacy
raw glyph/SVG APIs are unchanged. TrueType ignores VORG as required by OpenType.

### GSUB

- Parsed: `ScriptList`, `FeatureList`, `LookupList`
- Implemented: `lookup_vertical()` for single-substitution vertical forms
- Partial: `lookup_ccmp()` exists but does not expand all results yet
- Implemented: `lookup_locale()` and `lookup_liga()`
- Text APIs apply variation selectors and basic `locl` / `liga` / `dlig` / `ccmp`
- Direction-aware APIs support vertical flow and RTL layout
- Arabic-style RTL joining selects `isol` / `init` / `medi` / `fina` from Unicode Joining_Type (see limits below)
- Text shaping applies `rlig`, `rclt`, `calt`, and `clig` in both LTR and RTL modes when present
- Locale-aware lookup collection prefers matching scripts such as `arab`, `hebr`, and `syrc`
- Language-system selection also uses full locale subtags such as `ur-Arab-PK`
- Japanese variant forms can be requested through `FontOptions::font_variant`
- Context/chaining support is partially wired through the feature-sequence engine
- Not implemented: `lookup_width()`, `lookup_number()`

### Lookup parsing

- Type 1 Single Substitution: parsed and expandable
- Type 2 Multiple Substitution: parsed and expandable
- Type 3 Alternate Substitution: parsed and expandable
- Type 4 Ligature Substitution: parsed and expandable
- Type 5 Context Substitution:
  - Format 1 parsed, partially applicable
  - Format 2 parsed, partially applicable
  - Format 3 parsed, applicable
- Type 6 Chaining Context Substitution:
  - Format 1 parsed, partially applicable
  - Format 2 parsed, partially applicable
  - Format 3 parsed, applicable
- Type 7 Extension Substitution: parsed, not fully applied
- Type 8 Reverse Chaining Contextual Single Substitution: parsed, not applied

### GPOS Single Positioning

- Type 1 SinglePos Formats 1/2 and Type 9 wrapping SinglePos are parsed and executed.
- Default execution covers `kern` / `dist` horizontally and `vkrn` / `dist` / `vert` vertically, with the existing locale/script selection. Arbitrary feature selection and contextual GPOS are not added.
- All four static ValueRecord fields reach shaping and measurement, including explicit advances on attached marks. Distinct lookups accumulate in i32, and the first matching subtable (including a zero record) wins within one lookup.
- SinglePos observes GDEF glyph classes, IgnoreBaseGlyphs / IgnoreLigatures / IgnoreMarks, MarkAttachmentType, and MarkFilteringSet. PairPos shares this filtering. Ordinary mark attachment now filters the current lookup input only; target search and historical GDEF-origin fallback remain separate. See [bounded mark filtering](mark-input-filtering.md).
- Device / VariationIndex offsets are consumed but their deltas are not evaluated. Cursive positioning and complete Arabic shaping remain separate work.
- Regressions use the committed MIT synthetic font (`tests/fixtures/gpos-single.ttf`) and byte fixtures for truncation, invalid coverage/counts, lookup accumulation, locale, scaling, vertical flow, and mark attachment. Broad real-font / HarfBuzz comparison is not yet covered.
- Vertical `vert` selection also covers font-authored SinglePos for unclassified kana marks (including Extension lookups). It does not add `vert` PairPos, generic mark fallback, or anchor inference. Horizontal and sideways runs remain unchanged. Synthetic tests cover shared lookup deduplication, zero-subtable precedence, GDEF filtering, signed values, scaling, locale/required features and public shape/measure/render/family routes. Noto Sans CJK JP 2.004 provides a targeted real-font example; this is not general Japanese/HarfBuzz parity.
- Reference: [OpenType GPOS Single Adjustment](https://learn.microsoft.com/en-us/typography/opentype/spec/gpos#lookup-type-1-single-adjustment-positioning-subtable).

### GDEF

- Parsed: glyph class definitions, attach list, ligature caret list, mark attach class definition, mark glyph sets definition
- Current state: partially integrated
- Pair positioning now skips GDEF mark glyphs when searching previous/next spacing glyphs for kerning
- `GPOS mark-to-base` (Type 4 Format 1) is now parsed and used during shaping when a `mark` feature lookup is available
- `GPOS mark-to-mark` (Type 6 Format 1) is now parsed and used during shaping when an `mkmk` feature lookup is available
- The historical GDEF-based mark attachment heuristic remains only for fonts without GPOS. With GPOS present, absent anchors retain cursor/authored placement rather than synthesizing that heuristic.
- Static ligature carets are integrated into mapped caret layouts with explicit source provenance. Contour points are resolved only for unhinted static simple monochrome TrueType outlines; composite/COLR/variable points and Device/VariationIndex remain approximate. Bounds validation added here covers the ligature-caret path, not every historical GDEF subtable.
- Attach / mark-set data is still not fully integrated into higher-level layout

## Notes

- `FontFamily` currently supports cached-face selection and per-glyph fallback across loaded faces
- Family fallback chains and Last Resort handling are still not implemented
- Variable-font metadata and axis-driven metrics are available through `fvar` / `avar` / `HVAR` / `VVAR` / `MVAR`
- Public API axis entry points are `FontFace::variation_axes()` and `FontEngine::with_variation()`
- `gvar` outline deltas cover simple glyphs and XY-positioned composite glyphs, including recursive component variation; point attachment and inherited metrics remain limited as described above
- `gvar` phantom-point deltas now feed horizontal and vertical glyph metrics in both layout getters and shaping output
- Phantom-point behavior is covered by synthetic unit tests and Source Serif real-font regressions
- Parser hardening is in progress: malformed optional GSUB/GPOS feature-variation data is now skipped instead of panicking
- Unsupported contextual GSUB subtables now fall back to `LookupResult::None` instead of panicking during shaping
- `hmtx` / `vmtx` now tolerate zero-metric edge cases without panicking, returning zero advances as a fallback
- `OTFHeader`, `TTCHeader`, `get_font_type()`, and `COLR::new()` now return errors instead of panicking on truncated input
- `COLR::get_layer_record()` now stops at available layers instead of trusting malformed layer ranges
- `fontreader.rs` now removes more public-path `unwrap()` calls in name lookup, metrics/layout getters, and mandatory-table loading
- CFF2 outlines now load through the shared `cff.rs` path, including `vsindex` / `blend` evaluation and real-fixture SVG smoke coverage
- CFF2 variation support now covers both outline charstrings and Private DICT `vsindex` / `blend` parsing
- The current local corpus does not contain a confirmed real CFF2 font; coverage is therefore synthetic/unit-test heavy until a true CFF2 fixture is added
- With `svg-fonts`, OpenType `SVG ` glyphs are pathified first, and simple shapes are emitted as `GlyphLayer::Path`
- Payloads that cannot be pathified, and payloads that still contain unsupported constructs such as `pattern`, `mask`, or `filter`, are kept as `GlyphLayer::Svg` fallback layers
- The current supported SVG subset covers `path`, `rect`, `circle`, `ellipse`, `line`, `polyline`, `polygon`, `defs` / `use`, `fill` / `fill-rule` / `stroke` / `stroke-width`, `clipPath` / `clip-path`, simple `mask`, `translate` / `scale` / `rotate` / `skewX` / `skewY` / `matrix`, minimal `linearGradient` / `radialGradient` / `stop`, and preserved `gradientUnits` / `gradientTransform`
- Patterns, `mask`, `filter`, and richer stroke styles are still unsupported
- The `paintcore` bridge now preserves clip/gradient-capable layer data from the public 0.0.13 `fontcore` line
- WOFF2 still requires the complete byte stream before decoding
- CFF2 planning notes live in `cff2-investigation.md`

### Static CFF vertical origins

High-level shape, measurement and SVG export translate monochrome static CFF
outlines by half the horizontal advance width and the VORG default/per-glyph
signed Y origin. Origins are selected after substitution, shared by attached
marks (including mark-to-mark), and scaled once. Vertical advances still come
from vhea/vmtx; VORG does not supply advances.

Without VORG, the origin is the exact cubic outline yMax plus vmtx top side
bearing. Without usable vertical metrics, the outline is centered in the same
synthesized cell used for TrueType. Empty outlines use zero extrema. Public ink
bounds remain conservative (include control points); only origin calculation
uses exact curve extrema. sfnt, selected TTC faces and WOFF use this policy.
Invalid optional VORG (unsupported version, short data, unsorted or duplicate
records) is ignored and uses the same fallback. TrueType ignores VORG entirely.

This correction requires `cff`; sfnt vertical metrics require `layout` as before.
Color origins outside the static COLR v0 case below, BASE alignment and Unicode orientation remain outside
this scope. Horizontal and legacy raw APIs are unchanged.
Tests use committed MIT synthetic fixtures in `cff-vertical-origin.ttc` and
`cff-vertical-origin.woff`; provenance and SHA-256 are in `tests/fixtures/manifest.toml`.

### CFF2 variable vertical origins

With `cff2`, monochrome CFF2 uses the same high-level origin policy. A VORG
origin is the default instance value; only the explicitly mapped VVAR origin
delta is added at the selected normalized coordinates. Missing mappings mean
zero delta, not the implicit advance-height mapping. TSB variation is not added
to a VORG-derived origin. TrueType ignores both VORG and VVAR origin mappings.

Without usable VORG, origins use the actual coordinate-varied CFF2 cubic yMax
plus the coordinate-varied vmtx/VVAR top side bearing. Origin-map deltas are not
added to this fallback. Without vertical metrics, coordinate-varied extrema
are centered in the synthesized cell. Metric rounding/clamping follows the
existing vmtx/VVAR path; public ink bounds still include control points.

This covers default/intermediate/maximum coordinates, selected TTC faces, WOFF,
attached marks and scaling, including zero horizontal stretch. Coverage uses
MIT synthetic fixtures, not a confirmed real-world CFF2 vertical-font corpus.
Malformed optional VORG uses the fallback; general malformed CFF2/VVAR recovery,
color origins, BASE, Unicode orientation and legacy raw APIs are not extended.

See the [VORG specification](https://learn.microsoft.com/en-us/typography/opentype/spec/vorg)
and [VVAR specification](https://learn.microsoft.com/en-us/typography/opentype/spec/vvar).
Fixture provenance, reproduction and SHA-256: `tests/fixtures/manifest.toml`.

### Static TrueType COLR v0 vertical origins

With `layout` and vhea/vmtx, actually rendered COLR v0/CPAL path layers share the
base glyph's half horizontal advance and `glyf.yMax + vmtx.topSideBearing` origin.
An empty base has yMax zero: its TSB is the full origin, not an offset from the
colored outline union. This follows the [COLR glyph metrics requirements](https://learn.microsoft.com/en-us/typography/opentype/spec/colr#glyph-metrics-and-boundedness).
Layer paints and composition remain intact; attached monochrome/color marks
inherit the base translation. Font size/stretch apply once. Bounds, measurement
and SVG use the translated layers; horizontal and legacy raw APIs are unchanged.

This deliberately does not add origin correction for fonts with fvar (COLR layers
are not varied yet), COLR v1, CFF color outlines, sbix or SVG tables. Without
vertical metrics, including builds without `layout`, COLR offsets stay unchanged;
a rendered-union centering fallback is separate work. Missing CPAL or an unusable
palette that falls back to monochrome retains its previous origin policy rather
than treating COLR metadata as evidence of rendered color. Reproducible MIT cases
are in `colr-vertical-origin.ttc`; the generator and SHA-256 are in the manifest.

### Legacy vertical feature selection

`lookup_vertical()` selects `vrt2` when the existing collector finds at least
one valid referenced lookup; otherwise it selects `vert`. An empty feature or
one containing only invalid lookup indices is unavailable. Once `vrt2` is
selected, missing glyph coverage (including a lookup with no subtables) does
not fall back to `vert`. Unreferenced feature records do not count.
This legacy upright path never applies `vrtr`, because it does not graphically
rotate glyphs.
See the [OpenType vrt2 specification](https://learn.microsoft.com/en-us/typography/opentype/spec/features_uz#tag-vrt2)
for pre-rotated alternates and mutual exclusivity of these features.

This preserves the current global scope: all scripts, their default and named
LangSys records, and required features participate, including non-DFLT-only
fonts. It does not implement script-run selection or Unicode mixed orientation.
High-level text shaping resolves cmap/UVS and the existing locale prepass first.
LTR shaping merges the selected vertical feature with ordinary selected GSUB
features in LookupList order, once per lookup. RTL retains its composition and
joining boundary before the contextual/ligature/orientation pass. Metrics,
outline origins, and GPOS use the resulting final glyph IDs. Raw
`get_glyph_with_uvs_axis()` keeps its direct vertical alternate behavior without
running the text shaping stages. Source-cluster representation is unchanged.
Vertical single-feature lookup chaining is described below.
Builds without `layout` remain unchanged. MIT synthetic
fixtures verify selection, final glyph bounds/advance, measure, and SVG output;
see `tests/fixtures/manifest.toml` for provenance and hash.
`vertical-gsub-order.ttc` covers composition, variants, locale, ligatures, UVS,
final mark anchors and origins, selected-face fallback, and raw glyph compatibility.


### Legacy vertical lookup chaining

Within the selected vertical feature, direct Single substitutions (formats 1
and 2) and Extension-wrapped Single substitutions run once per referenced
lookup, in LookupList order. Feature indices can be stored in arbitrary order;
references repeated across features or LangSys records do not repeat a lookup.
Each lookup uses the preceding lookup's output and stops at its first matching
subtable, including identity substitutions. An applied identity or net cycle
returns `Some(original_id)`, while no applicable substitution returns `None`.

This single-glyph API skips Multiple, Alternate, Ligature, contextual and reverse
contextual lookups, including Extension wrappers of those types. It does not
flatten their results or discard context to manufacture a single replacement.
Unsupported/empty lookups do not trigger per-glyph `vert` fallback. The scalar API retains its global Script/LangSys scope. Text APIs use the
sequence engine described below; this scalar API remains deliberately limited.

See [OpenType common layout: feature and lookup processing](https://learn.microsoft.com/en-us/typography/opentype/spec/chapter2#feature-table)
and [GSUB substitution types](https://learn.microsoft.com/en-us/typography/opentype/spec/gsub).
The reproducible MIT `vertical-lookup-chain.ttc` tests cover ordering, identity,
cycles, duplicate references, unsupported types, and final bounds/origin,
advance, measurement and SVG; its generator and SHA-256 are in the manifest.

This single-glyph API has no GDEF class or mark-filtering context. A lookup
using IgnoreBaseGlyphs, IgnoreLigatures, IgnoreMarks, UseMarkFilteringSet, or
MarkAttachmentType is therefore skipped entirely. This conservative unsupported
case prevents substitutions on excluded glyphs; it is not full GDEF filtering
and does not trigger `vert` fallback. RightToLeft is ignored for GSUB by the
OpenType specification and does not prevent substitution. Direct and Extension
fixtures cover each filtering flag in standalone and chained lookups.

### Sideways LTR runs

`FontEngine::shape_sideways_ltr`, `measure_layout_sideways_ltr`,
`measure_sideways_ltr`, and `render_svg_sideways_ltr` are explicit, single-face,
single-line operations. They compose horizontal LTR text, then rotate the whole
run 90 degrees clockwise. Each call overrides the engine's LTR, RTL, or
top-to-bottom policy without mutating the engine or its options. Style, locale,
variant, supported variation coordinates, font size, horizontal stretch, and
SVG units are retained. No persistent mode, required public fields, or public
enum variants are added; the existing generic and legacy raw APIs are unchanged.

With `layout`, `vrtr` joins the ordinary selected LTR GSUB lookups in
LookupList order, followed by metrics and GPOS using the resulting glyph IDs. Sideways calls never
select `vert` or `vrt2`, and a missing or unsupported `vrtr` substitution does
not fall back to either feature. Without `layout`, rotation still works but
OpenType layout substitution and positioning are unavailable.

The `vrtr` collector retains the existing all-Script/all-LangSys scope. Its
single-glyph lookup chain supports direct and Extension-wrapped Single
substitutions in LookupList order, once per referenced lookup. Sequence-aware
substitutions and GDEF-dependent filtering flags are skipped, as described for
legacy vertical lookup chaining above. Ordinary horizontal shaping keeps its
existing GSUB/GPOS sequence and filtering limitations. Pair-GPOS `yPlacement`
is converted from the font's y-up coordinates to the run's y-down coordinates
before sideways rotation, as in ordinary horizontal and upright vertical runs.
Existing `yAdvance` behavior is unchanged. This is not complete OpenType shaping
or Unicode mixed-orientation layout.

Horizontal metrics supply advances and origins; this path does not use `vmtx`
or `VORG` for vertical layout. In y-down coordinates, positions, paths, offsets,
and advance vectors use `(x, y) -> (-y, x)` about baseline `(0, 0)`, with no
em-centering translation. Bounds are rotated too. Font ascent, descent, and
line gap remain the horizontal values as cross-axis references, while
`FontMetrics::flow` becomes `GlyphFlow::Vertical` (ascent to the right,
descent to the left). Legacy bearings describe an LSB/positive-up ascender
reference, so `(bearing_x, bearing_y) -> (bearing_y, -bearing_x)`; they are not
the ink location. Use the transformed bounds for ink placement.

Measurement and SVG consume the same transformed run. `inline_advance` is the
absolute sum of y advances; `measure_sideways_ltr` returns that scalar.
`ink_bounds` unions positioned glyph bounds. `line_bounds` unions each
positioned origin and its endpoint using both components of the rotated
advance vector. `block_extent` is the x extent of `line_bounds`, not a font
line height. Empty runs have no bounds and zero extents; space-only runs retain
their rotated horizontal advances. SVG uses the usual ink padding and the
engine's SVG unit.

Acceptance depends on the actual layers, not a font-format whitelist: every
layer must be a clip-free `GlyphLayer::Path` with `GlyphPaint::CurrentColor`
and `PathPaintMode::Fill`. A compatible pathified SVG or foreground-only COLR
layer can qualify. Solid colors, gradients, strokes, clips, raster layers, and
raw SVG layers return `std::io::ErrorKind::Unsupported`, without partial output.
TAB, CR, LF, VT, FF, NEL, U+2028, and U+2029 return
`std::io::ErrorKind::InvalidInput`. Family fallback, BiDi, automatic Unicode
direction/orientation, tabs, multiline layout, and baseline alignment with
upright CJK remain outside this API.

The design follows [CSS Writing Modes: vertical font features](https://www.w3.org/TR/css-writing-modes-4/#vertical-font-features)
and [OpenType `vrtr`](https://learn.microsoft.com/en-us/typography/opentype/spec/features_uz#tag-vrtr)
for graphical rotation with rotation-specific alternates. These references do
not imply support for full CSS layout. See [API Recipes](api-recipes.md#sideways-ltr-shaping-measurement-and-svg)
for the public calls.

## Layout controls next to ZWJ

The shared text parser keeps LF and TAB as layout controls, including directly
after U+200D ZERO WIDTH JOINER. CR is ignored as before, so CRLF produces one
newline. This applies to horizontal and upright vertical shaping and family
fallback: LF starts a new line/column and TAB advances by four line heights.
Controls do not turn into missing-glyph advances or enter fallback clusters.
Existing glyph-advance and family maximum-line measurement contracts are unchanged.

This retains the existing CR/LF/TAB layout policy. Unicode extended grapheme
boundaries are described separately below; other Unicode controls are not
newly treated as line breaks. Tests reuse committed MIT fonts.

## Pair positioning coordinates

GPOS PairPos formats 1 and 2 keep their signed font-unit records unchanged.
The high-level run converts scaled `yPlacement` to y-down once, for both value
records, before adding it to the pen or attachment origin. Positive placement
therefore raises ink in LTR, RTL, and upright TTB; sideways rotates the corrected
horizontal placement. X placement/advance, mark attachment, and legacy raw
command APIs are unchanged.

Advance is directional, not a Cartesian placement vector: the
[OpenType GPOS specification](https://learn.microsoft.com/en-us/typography/opentype/spec/gpos#overview)
explicitly states that positive `YAdvance` increases top-to-bottom advance
height. It is added to the positive screen-down `vmtx` advance without another
sign flip. Existing horizontal cross-axis advance handling and axis-only
ordinary line bounds are preserved; this is not a claim of complete GPOS
axis applicability or sequence semantics.

The MIT `pair-coordinates.ttc` fixture tests both PairPos formats, positive,
negative, and zero Y values on both glyphs, differing horizontal/vertical
metrics, size/stretch, raw values, LTR/RTL/TTB positions and advances, ink and
line measurement, SVG, and sideways rotation. Regenerate it with
`python tests/fixtures/generate_pair_coordinates.py` (fontTools 4.65.0).

## Extended grapheme boundaries

Shaping and FontFamily fallback share Unicode **17.0.0** extended grapheme
clusters from the exactly pinned `unicode-segmentation = "=1.13.3"`. This replaces
the handwritten mark ranges and unconditional ZWJ grouping. Hangul L/V/T,
SpacingMark, Prepend, Indic conjuncts (GB9c), emoji ZWJ sequences, and regional
indicator pairs follow UAX #29. For example, `A + ZWJ + B` is now two units
(`A + ZWJ`, then `B`), not an arbitrary ZWJ ligature; an intervening combining
mark also prevents pairing two regional indicators across that mark.

The layout adapter still ignores lone CR, emits one newline for CRLF or LF,
and treats TAB as a tab. NUL, VT, FF, NEL, U+2028, U+2029 and other controls
keep their existing scalar-glyph behavior rather than becoming line breaks.
Leading orphan variation selectors are ignored, including repeated selectors,
but following marks in that same grapheme are preserved. Base+selector UVS
lookup and scalar ZWJ/.notdef handling are unchanged.

Fallback selects one face for each adapted cluster. If no candidate supports
the entire cluster, the existing first-candidate/.notdef policy is retained;
it does not splice a cluster across partial fonts. Segmentation does not itself
perform Indic/emoji shaping, guarantee a single glyph, normalize text, or add
BiDi. Raw HTML helpers retain all scalar glyphs in two-character graphemes,
instead of assuming every such pair is a base+selector; their existing
multi-scalar SVG rendering path and vertical-layout limitations remain.

This boundary version is independent of the public **Unicode 18.0.0**
`Vertical_Orientation` scalar property. The boundary iterator itself does not
choose orientation or provide a source-provenance model or CSS conformance.
The explicit mixed-layout methods described below perform orientation grouping.
See [Unicode sources, licenses, and offline verification](../data/unicode/README.md#extended-grapheme-clusters-unicode-1700).

## Opt-in mixed vertical LTR outlines

`FontEngine::shape_vertical_mixed_ltr`, `measure_vertical_mixed_ltr`,
`measure_layout_vertical_mixed_ltr`, and `render_svg_vertical_mixed_ltr` are
explicit per-call, single-face, single-line LTR operations. They override the
engine's selected direction without changing its options or existing methods.
No public options field, direction variant, or glyph structure was added.

The boundary algorithm is **Unicode 17.0 EGC** (`unicode-segmentation 1.13.3`),
while **Unicode 18.0 VO** and **Unicode 18.0 General_Category=Me** are separately
pinned property tables. Before any GSUB, each original EGC uses its first
scalar's VO; any enclosing mark (`Me`, 13 scalars in five ranges) forces that
entire EGC upright. `U`, `Tu`, and `Tr` form upright groups, and `R` forms
sideways groups. Adjacent EGCs with the same final orientation are coalesced,
so ordinary GSUB, ligatures, kerning and mark attachment work within each group.
GSUB/GPOS deliberately do not cross orientation boundaries. This avoids inferring
orientation from substituted glyph IDs or incomplete substitution provenance.

Upright groups use existing vertical metrics/origins and select **only `vert`**
together with ordinary GSUB in LookupList order, before final metrics and GPOS. They never select `vrt2` or
`vrtr`. `Tr` with no applicable `vert` alternate, including identity substitution,
stays upright; rotated fallback synthesis is not implemented. Sideways groups
use horizontal LTR metrics/GPOS and **only `vrtr`**, then the existing clockwise
whole-run transformation. The ordinary vertical API still selects `vrt2` when
available, otherwise `vert`; it is not silently changed by these methods.
Without `layout`, the same grouping and geometry apply without GSUB/GPOS.

Each sideways group is centered on the vertical axis using the horizontal
`FontMetrics` from that exact shaped instance: `x_shift = -(ascent - descent) / 2`,
where descent is a positive distance. These values are already scaled and retain
existing supported variations, including the current MVAR metric path. The shift
is added only to positioning, not advances. Stretch is not applied again; ink
bounds, line gap and requested line height are not substituted for the metric
midpoint. Existing font-size scaling can itself include the font's line gap.
This is fontcore synthesis, not OpenType BASE, OS/2 metric priority, or CSS
font-size/baseline equivalence. Groups connect by summing **both components** of
all final glyph advance vectors; GPOS placement, last drawn position, ink edges
and centering shifts never become the next group's pen. Existing GPOS advance
behavior is preserved rather than broadened into full OpenType shaping.

All groups must contain only CurrentColor, clip-free fill paths. Unsupported
colors, gradients, strokes, clips, raster or raw SVG layers fail atomically with
`Unsupported`; no partial run is returned and the engine remains usable.
TAB, CR, LF, VT, FF, NEL, U+2028 and U+2029 are rejected with `InvalidInput`.
Empty runs have zero extents and no bounds; spaces retain their rotated advances.
Measurement uses the final run, and line bounds include every positioned origin
and full advance-vector endpoint. SVG exports precisely that run with normal
ink padding and the configured SVG unit.

This is an explicit **UAX #50-style first-scalar/any-Me policy**, not complete
CSS character tailoring. [CSS Writing Modes mixed orientation](https://www.w3.org/TR/css-writing-modes-4/#mixed-orientation)
and [CSS Text Appendix E](https://www.w3.org/TR/css-text-3/#characters) use
additional typographic-character/base tailoring. No Unicode18 EGC, complete CSS,
browser-equivalent layout, BiDi, family fallback, multiline/tabs, BASE alignment,
or complete OpenType behavior is claimed. See also
[UAX #50 revision 35](https://www.unicode.org/reports/tr50/tr50-35.html) and
[the separate Unicode data/version provenance](../data/unicode/README.md).

The deterministic MIT fixture covers Han+AV+Han, both cross-axis advances and
signed placements, asymmetric hhea metrics and gaps, stretch, MVAR, Tr feature
sentinels, ccmp/multiple/ligature/mark ordering, and the Unicode17/18 boundary
difference `A + U+1D127`. Existing fixtures cover selected TT/CFF/CFF2 faces,
missing vertical metrics, supported variations and rejected color layers.
Regenerate with `python tests/fixtures/generate_mixed_vertical_ltr.py`
(fontTools 4.65.0); tests need only committed bytes, not Python or installed fonts.

## Explicit tate-chu-yoko LTR cells

`shape_tate_chu_yoko_ltr`, `measure_tate_chu_yoko_ltr`,
`measure_layout_tate_chu_yoko_ltr`, and `render_svg_tate_chu_yoko_ltr` compose an
explicitly selected single-face, single-line string horizontally into one
upright synthetic glyph. They override direction for that call without changing
the engine. Ordinary horizontal GSUB/GPOS, style, locale, variants and supported
variation coordinates are retained; `vert`, `vrt2` and `vrtr` are not selected.
Existing horizontal and mixed-vertical APIs are unchanged.

The cell is one scaled em: `units_per_em * horizontal_outline_scale`, using
exactly the same instance-dependent scale as horizontal shaping. This matters
when units-per-em differs from the horizontal ascender/descender/line-gap sum:
`font_size` itself is a line-height input in the existing API, not one em.
Supported MVAR changes affect this scale once. Stretch and explicit line height
do not change the cell. Its origin is the top center, advance is `(0, cell_size)`,
and logical bounds are `[-cell_size/2, 0, cell_size/2, cell_size]`. Source glyphs become separate path
layers of one composite glyph; original glyph boundaries are not retained.
Its synthetic font metrics are half-cell ascent/descent, zero gap and vertical
flow, with zero bearings. The cell is a low-level composition primitive, not
an automatic insertion into `shape_vertical_mixed_ltr`.

Let `size` denote the scaled em, `W` the sum of the horizontal run's x advances,
and `S = min(1, size/W)`
(`S = 1` when W is zero). Final outline x is `S * (glyph.x + layer.x + x - W/2)`.
The horizontal baseline is placed at `size/2 + (ascent-descent)/2`, using the
already-scaled horizontal font metrics and positive descent. Glyph y placement,
path y and layer y are preserved relative to that baseline. This is metric-based
centering, not ink centering. Line height does not change it. Horizontal stretch
and font size apply before fitting; no enlargement is performed. Kerning affects
the fitting width; internal cross-axis advances affect positioned source ink
but not the final cell advance.

Only advance width is fitted. Overhanging ink/marks can exceed the square and
are not clipped. Leading/trailing spaces affect centering; all spaces are
preserved. Spaces and zero-advance marks occupy a cell when shaping emits a
glyph; empty input and ignored orphan selectors with no glyphs occupy none.
Measurement keeps conservative transformed ink bounds separate from the square.
SVG uses the exact final run and normal ink padding, so ink-free cells produce
zero-sized empty SVG despite their nonzero logical advance.

As with the explicit sideways API, only current-color, clip-free filled paths
are accepted. Other colors/gradients, strokes, clips, raster and raw SVG layers
return `Unsupported`. Tabs and line-break controls, invalid options, negative
total horizontal advance or non-finite composed geometry return `InvalidInput`.
No partial output is returned. Path layer ordering and fill rules are preserved.

The [CSS Writing Modes horizontal-in-vertical section](https://www.w3.org/TR/css-writing-modes-4/#text-combine-upright)
is background for the composition concept, not a conformance claim. Automatic
digit ranges, width variants (`hwid`/`twid`/`qwid`), full-width normalization,
BiDi, fallback, BASE alignment, multiline flow and line-breaking rules are not
implemented here. Existing horizontal shaping limitations still apply.
MIT fixtures, generator and SHA-256 are in `tests/fixtures/manifest.toml`;
`tests/tate_chu_yoko.rs` covers the public APIs without installed fonts.

### Lookup-aware PairPos filtering

PairPos formats 1/2 and their Type 9 Extension wrappers now select neighbors
inside each lookup, using GDEF IgnoreBaseGlyphs, IgnoreLigatures, IgnoreMarks,
MarkAttachmentType, and MarkFilteringSet. A zero flag no longer skips every
mark. IgnoreMarks takes precedence over mark sets, which take precedence over
attachment classes. With no GDEF, glyph classes are not inferred. Tabs and
newlines terminate pair matching. Existing locale/script selection and kern /
vkrn feature selection are unchanged; this does not add full GPOS shaping.

The first matching subtable wins even for an all-zero record or class-0 matrix
entry. Distinct lookups accumulate both ValueRecords in i32, with no intermediate
i16 saturation. ValueFormat2 presence controls the next scan position: an empty
second record permits overlapping pairs, while a present record consumes the
second glyph even when all its values are zero. SinglePos still accumulates with
pairs, and explicitly positioned attached marks keep their advance adjustments.
The pre-existing GDEF/GPOS mark attachment path is unchanged and is not newly
lookup-filter-aware. Device/VariationIndex adjustments remain unevaluated.

The reproducible MIT `pair-filtering.ttc` and byte tests cover filtering flags
and their precedence, absent GDEF, both formats and extensions, first-match zero
records, distinct lookups with different neighbors, wide accumulation, cursor
advancement, locale, boundaries, both ValueRecords, LTR/RTL/vertical placement,
non-unit size/stretch, measurement, and an exact SVG viewBox with attachment.
Existing pair-coordinate tests are unchanged except for their internal helper
call. Broad real-font/HarfBuzz parity remains unverified.

References: [GPOS pair adjustment](https://learn.microsoft.com/en-us/typography/opentype/spec/gpos#lookup-type-2-pair-adjustment-positioning-subtable)
and [lookup flags](https://learn.microsoft.com/en-us/typography/opentype/spec/chapter2#lookupFlags).

## Explicit composite font selection

Added in the 0.0.18 source tree: `CompositeFont`, `CompositeFontRule`
and `UnicodeRange` are additive types with private fields. They do not add
variants/fields to existing `FontRef`, `FontOptions`, descriptors or glyphs.

- Inclusive scalar ranges, nonempty range sets and ordered face lists; reversed
  endpoints and empty lists return `InvalidInput`.
- Unicode 17.0 EGC boundaries and the existing parser representative are reused.
  Prepend remains the representative; only it must match the range. Marks,
  Hangul/Indic clusters, IVS and ZWJ sequences are not split between faces.
- The first matching rule owns the cluster. Whole-cluster support is tested in
  face-list order; on total failure its first face keeps `.notdef`. No later
  rule or default list is tried. Existing support may accept a base glyph with
  no requested UVS mapping; later UVS/color faces are not preferred. Coverage
  does not promise complete contextual, Indic or emoji shaping.
- Adjacent clusters choosing the same borrowed face object and exactly equal
  size multipliers and baseline shifts share a shaping span even across rule
  boundaries. Face or adjustment changes stop GSUB/GPOS context. A cloned face
  is a distinct object. Both x and y advances carry across spans.
- Size retains existing face-specific horizontal line-metric normalization
  (ascender minus descender plus line gap, including supported variations),
  not a shared UPEM normalization. Common stretch/locale/variant/axes are
  forwarded. Each rule's positive finite `with_size_multiplier` (default `1.0`)
  multiplies the requested nominal size before the normal engine shapes the span.
- `with_baseline_shift` accepts finite signed engine-output units (default `0.0`).
  Positive shifts move glyphs upward by decreasing y; they are not scaled by size
  and do not alter advances or the following cursor. Both adjustment builders
  return `Result<Self, std::io::Error>`, with `size_multiplier()` and
  `baseline_shift()` getters. Local fallback uses its owning rule's adjustments;
  unmatched clusters retain multiplier `1.0` and shift `0.0`.
- Only single-line horizontal LTR shape/measure/SVG is exposed. TAB, CR, LF,
  VT, FF, NEL, U+2028 and U+2029 return `InvalidInput`. There is no vertical,
  RTL/BiDi, wrapping, mixed-orientation or tate-chu-yoko composition API.
- Measurement is the sum of horizontal advances, not ink width. SVG uses the
  same composed run and existing exporter behavior. No feature is required;
  optional shaping/color support remains governed by the existing features.

Synthetic tests reuse the repository's MIT EGC, PairPos and ligature fixtures.
Existing family fallback and horizontal/vertical APIs are unchanged.

Composite shaping rejects invalid or overflowing effective sizes. Runs using
non-default adjustments also reject nonfinite transformed geometry and advances
rather than saturating values. Identity-only runs retain the existing geometry
behavior, including possible overflow for extreme inputs. These checks do not
change the legacy face/family engines' numeric semantics. Vertical, multiline
and RTL composition remain future work.

Real-font spot checks are not a general shaping guarantee. The integrated
contextual sequence path applies selected LTR `rclt` and contextual orientation
lookups as described below, including authored cleanup order. Composite size and
baseline adjustments do not extend that shaping scope or imply full
[Furifont](https://furifont.antfu.me/) support.

### Contextual sequence shaping (0.0.18)

LTR text applies selected `ccmp`, variants, `rlig`, `rclt`, `calt`, `clig`, `liga`
and the route's single orientation feature in deduplicated LookupList order.
The orientation lookup scope remains all-script/all-LangSys, independently of
ordinary locale selection. RTL retains composition/variant then joining before
its contextual/ligature/orientation pass. This is a bounded shaping policy, not
full script itemization or HarfBuzz parity. The legacy locale prepass is unchanged.

Context and chaining formats 1/2/3, including nested Single, Multiple and
Ligature substitutions, retain source indexes and ligature component counts for
GPOS. Format 1 now reads both fields of every SequenceLookupRecord. Matched
zero-action rules block later rules; reverse chaining also works through an
Extension wrapper. Newline, tab and mixed-orientation group boundaries still
stop sequence matching. Alternate selection and general glyph reordering are
not added.

A font can deliberately place contextual replacement before `ccmp` cleanup, as
Furifont v0.7.0 does for IVS placeholders. This implementation honors the selected
LookupList order rather than forcing orientation after cleanup. OpenType's
`vrt2` recommendation to be the last substitution **in the font** is a font
ordering recommendation; this engine does not reorder an authored LookupList to
enforce it. It still selects `vrt2` when available, otherwise `vert`, never both;
mixed upright uses only `vert`, sideways only `vrtr`. See [LookupList processing](https://learn.microsoft.com/en-us/typography/opentype/spec/chapter2#lookuplist-table)
and [vertical alternates and rotation](https://learn.microsoft.com/en-us/typography/opentype/spec/features_uz#tag-vrt2).

Outside the [bounded Tibetan filtered-context exception](../docs/filtered-context.md),
GDEF-dependent flags do not support matching across excluded glyphs. A
flagged sequence lookup is permitted only when a declared GDEF class definition
proves that no glyph in the current run would be excluded; mark attachment or
filter-set flags conservatively require no declared marks. Otherwise the lookup
is skipped, including nested lookups. This keeps ordinary flagged ligatures in
mark-free runs without ignoring flags. Raw scalar vertical APIs still skip all
GDEF-dependent flags. Malformed recursion is bounded at 32 nested lookups and
4096 nested operations per root position; expanding sequences are capped at
1,000,000 glyphs. Hitting a limit stops that substitution without a public error.

## Bounded residual kana mark advances

With `layout`, exact original kana-plus-U+3099/U+309A graphemes can suppress an
unpositioned nominal mark advance in GPOS faces without GDEF glyph classes.
Authored positioning and GSUB decisions remain authoritative. This is not
attachment repair: the mark can touch or overlap the following glyph. See the
[precise admission policy and exclusions](kana-residual-marks.md).

## Unicode-based Arabic-style joining

Explicit RTL text shaping analyzes original logical-order scalars with pinned
Unicode 17.0.0 Joining_Type data before ccmp. Dual/right/left joining, transparent
marks, tatweel, ZWJ and ZWNJ determine joining independent of which form lookups
the font supplies. Source-index metadata carries the selected form through
ccmp expansion or contraction; missing selected forms retain the glyph rather
than falling back to a different joining form. Transparent and join-causing
characters do not themselves receive joining-form substitutions.

This fixes the basic OT path, not complete Arabic shaping. Joining substitution
retains the existing single-result feature helper, not a general contextual or
multi-lookup form stage. GDEF-filtered GSUB skipping, marked required ligatures,
cursive positioning, all mark positioning, normalization, default-ignorable
glyph removal and Syriac joining-group rules remain outside this change.
ZWJ/ZWNJ joining semantics are corrected, but their glyphs can still draw in
some fonts despite zero advance. SFArabic marked samples can retain mark-position
differences from HarfBuzz; matching advances alone does not validate ink placement.
Source-index inheritance does not implement general cluster reconstruction.
RTL positioning is a caller-selected run direction, not full Unicode bidi or
script itemization: mixed Latin/digits need externally resolved directional runs.
AAT-only fonts (for example macOS Geeza Pro with morx and no GSUB) do not gain
OpenType shaping from this change. LTR and layout-disabled behavior is unchanged.

Reproducible tests include every scalar's Joining_Type, missing font forms,
mark/control boundaries, and a synthetic public shaping fixture with actual
ccmp expansion and contraction. A bounded local comparison against HarfBuzz 15
with Arial and SFArabic covered ordinary words, marks and tatweel; it is not a
claim of all-font or all-position equivalence. The real comparison font binaries are not distributed.

References: [Unicode data](https://www.unicode.org/Public/17.0.0/ucd/extracted/DerivedJoiningType.txt),
[OpenType Arabic shaping](https://learn.microsoft.com/en-us/typography/script-development/arabic).

## Bounded mark input filtering

GPOS types 4/5/6 and ExtensionPos now apply GDEF filtering to the current input,
with explicit target-search, missing-class and synthetic-fallback limits.
See [scope](mark-input-filtering.md) and [regression commands](testing.md#bounded-mark-input-filtering).

### Bounded Old Hangul Standard vertical stage

Exact old/extended L V [T] runs may additionally use Standard top-to-bottom
shaping when the selected hang Script/LangSys contains the globally selected
vert/vrt2 feature. Mixed/sideways/RTL/tone runs remain excluded. This does not
force Jamo vertical advance to zero: existing vmtx/VORG and authored GPOS vert
SinglePos still control placement. See [scope and proof](bounded-vertical-hangul.md).
