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
CFF2 VORG, VVAR origin maps, BASE alignment, Unicode orientation, and color
origins (COLR, sbix, SVG) remain separate work. Color layer offsets and the legacy
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
CFF2/VVAR origin variation, color COLR/sbix/SVG origins, BASE alignment and Unicode
orientation remain outside this scope. Horizontal and legacy raw APIs are unchanged.
Tests use committed MIT synthetic fixtures in `cff-vertical-origin.ttc` and
`cff-vertical-origin.woff`; provenance and SHA-256 are in `tests/fixtures/manifest.toml`.
