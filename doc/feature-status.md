# Feature Status

This document keeps the implementation-oriented notes that were previously in `README.md`.
The README now focuses on the public API and runnable examples.

## Layout support

`layout` is partially implemented.

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
- Attach / caret / mark-set data is still not integrated into higher-level layout

## Notes

- `FontFamily` currently supports cached-face selection and per-glyph fallback across loaded faces
- Family fallback chains and Last Resort handling are still not implemented
- Variable-font metadata and axis-driven metrics are available through `fvar` / `avar` / `HVAR` / `VVAR` / `MVAR`
- Public API axis entry points are `FontFace::variation_axes()` and `FontEngine::with_variation()`
- `gvar` outline deltas now cover both simple glyphs and composite glyphs, including recursive component variation for Source Serif variable-font fixtures
- `gvar` phantom-point deltas now feed horizontal and vertical glyph metrics in both layout getters and shaping output
- Phantom-point behavior is covered by synthetic unit tests and Source Serif real-font regressions
- Parser hardening is in progress: malformed optional GSUB/GPOS feature-variation data is now skipped instead of panicking
- `hmtx` / `vmtx` now tolerate zero-metric edge cases without panicking, returning zero advances as a fallback
- `OTFHeader`, `TTCHeader`, `get_font_type()`, and `COLR::new()` now return errors instead of panicking on truncated input
- `COLR::get_layer_record()` now stops at available layers instead of trusting malformed layer ranges
- CFF2 outlines now load through the shared `cff.rs` path, including `vsindex` / `blend` evaluation and real-fixture SVG smoke coverage
- CFF2 variation support now covers both outline charstrings and Private DICT `vsindex` / `blend` parsing
- The current local corpus does not contain a confirmed real CFF2 font; coverage is therefore synthetic/unit-test heavy until a true CFF2 fixture is added
- SVG glyph layers still return `ErrorKind::Unsupported`
- WOFF2 still requires the complete byte stream before decoding
- CFF2 planning notes live in `cff2-investigation.md`
