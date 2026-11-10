# Feature Status

This document keeps the implementation-oriented notes that were previously in `README.md`.
The README now focuses on the public API and runnable examples.

## Layout support

`layout` is partially implemented.

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
orientation and punctuation rotation remain separate work. Static CFF origins are covered below.

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
This correction is vertical-only; horizontal `HVAR`/`gvar` interaction is not
changed or validated by these fixtures.

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
BASE alignment, Unicode orientation, and color
origins outside the static COLR v0 case below remain separate work. The legacy
raw glyph/SVG APIs are unchanged. TrueType ignores VORG as required by OpenType.

### GSUB

- Parsed: `ScriptList`, `FeatureList`, `LookupList`
- Implemented: `lookup_vertical()` for single-substitution vertical forms
- Partial: `lookup_ccmp()` exists but does not expand all results yet
- Implemented: `lookup_locale()` and `lookup_liga()`
- Text APIs apply variation selectors and basic `locl` / `liga` / `dlig` / `ccmp`
- Direction-aware APIs support vertical flow and RTL layout
- Arabic shaping currently covers `isol` / `init` / `medi` / `fina`
- Arabic shaping also applies `rlig`, `rclt`, `calt`, and `clig` when present
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
- Default execution covers `kern` / `dist` horizontally and `vkrn` / `dist` vertically, with the existing locale/script selection. Arbitrary feature selection and contextual GPOS are not added.
- All four static ValueRecord fields reach shaping and measurement, including explicit advances on attached marks. Distinct lookups accumulate in i32, and the first matching subtable (including a zero record) wins within one lookup.
- SinglePos observes GDEF glyph classes, IgnoreBaseGlyphs / IgnoreLigatures / IgnoreMarks, MarkAttachmentType, and MarkFilteringSet. This does not add shared filtering to the existing pair/mark paths.
- Device / VariationIndex offsets are consumed but their deltas are not evaluated. Cursive positioning and Unicode Arabic joining remain separate work.
- Regressions use the committed MIT synthetic font (`tests/fixtures/gpos-single.ttf`) and byte fixtures for truncation, invalid coverage/counts, lookup accumulation, locale, scaling, vertical flow, and mark attachment. Broad real-font / HarfBuzz comparison is not yet covered.
- Reference: [OpenType GPOS Single Adjustment](https://learn.microsoft.com/en-us/typography/opentype/spec/gpos#lookup-type-1-single-adjustment-positioning-subtable).

### GDEF

- Parsed: glyph class definitions, attach list, ligature caret list, mark attach class definition, mark glyph sets definition
- Current state: partially integrated
- Pair positioning now skips GDEF mark glyphs when searching previous/next spacing glyphs for kerning
- `GPOS mark-to-base` (Type 4 Format 1) is now parsed and used during shaping when a `mark` feature lookup is available
- `GPOS mark-to-mark` (Type 6 Format 1) is now parsed and used during shaping when an `mkmk` feature lookup is available
- When neither `GPOS mark-to-base` nor `mark-to-mark` anchors are available, shaping still falls back to the existing GDEF-based mark attachment heuristic
- Attach / caret / mark-set data is still not fully integrated into higher-level layout

## Notes

- `FontFamily` currently supports cached-face selection and per-glyph fallback across loaded faces
- Family fallback chains and Last Resort handling are still not implemented
- Variable-font metadata and axis-driven metrics are available through `fvar` / `avar` / `HVAR` / `VVAR` / `MVAR`
- Public API axis entry points are `FontFace::variation_axes()` and `FontEngine::with_variation()`
- `gvar` outline deltas now cover both simple glyphs and composite glyphs, including recursive component variation for Source Serif variable-font fixtures
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
High-level text shaping resolves cmap/UVS first and applies the selected vertical
feature after ordinary GSUB stages (composition, locale/variant, RTL forms, and
ligatures). Metrics, outline origins, and GPOS use those final glyph IDs. Raw
`get_glyph_with_uvs_axis()` keeps its direct vertical alternate behavior without
running the text shaping stages. Source-cluster representation is unchanged.
Vertical single-feature lookup chaining is described below.
Horizontal flow and builds without `layout` remain unchanged. MIT synthetic
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
Unsupported/empty lookups do not trigger per-glyph `vert` fallback. The global
Script/LangSys scope and generic non-vertical feature helpers are unchanged.
This is not a full sequence-aware vertical shaping implementation.

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

With `layout`, ordinary horizontal GSUB runs first, followed by `vrtr`, then
metrics and GPOS using the resulting glyph IDs. This ordering is an
implementation choice, not a specification requirement that `vrtr` run last;
the explicit last-feature requirement applies to `vrt2`. Sideways calls never
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
