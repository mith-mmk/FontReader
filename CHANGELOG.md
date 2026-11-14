# Changelog

## 0.0.17 - 2026-11-14

### Added

- Explicit LTR tate-chu-yoko cell APIs: `shape_tate_chu_yoko_ltr`, `measure_tate_chu_yoko_ltr`, `measure_layout_tate_chu_yoko_ltr`, and `render_svg_tate_chu_yoko_ltr`. They fit a single horizontal run into one upright, scaled-em cell without enlarging it or changing the engine's shaping policy.
- Run all-feature library, integration, and documentation tests in CI on stable Rust and the Rust 1.91 minimum, including feature-gated CFF2, SVG-font, and raw API regressions.

### Fixed

- Apply GPOS PairPos formats 1/2 and Extension lookups with per-lookup GDEF filtering and neighbor selection. Preserve first-match zero records and class-0 matches, accumulate both ValueRecords across distinct lookups without intermediate i16 saturation, and advance the scan cursor according to ValueFormat2 presence. Existing SinglePos and attached-mark advance adjustments are retained.
- Apply horizontal HVAR and gvar metric deltas once, using HVAR advance/LSB entries when available, retaining gvar bearings for a missing optional LSB map, and respecting authoritative mapped-zero values. Cover implicit and explicit advance mappings, including nonidentity maps.
- Apply composite-glyph gvar tuples to component indices and phantom points rather than flattened outline points. Preserve sparse component semantics, selected-face metrics, child-only variations, per-field HVAR/VVAR precedence, and fractional nested XY offsets until the final integer outline conversion. Do not treat `ROUND_XY_TO_GRID` as design-unit rounding in the unhinted path.

### Compatibility

- Preserve existing public signatures, public structs/enums, feature flags, dependencies, licensing, and the Rust 1.91 minimum; the four tate-chu-yoko methods are additive. Corrected positioning, outlines, advances, and bearings can change rendered output, measurement, and SVG bounds.
- This is a version boundary before multi-font composition APIs. Composite glyphs within one face are not composite fonts or multi-face fallback.
- Update dependency requirements explicitly: Cargo's `fontcore = "0.0.16"` requirement does not select 0.0.17.

### Verification and limits

- Add self-contained MIT synthetic fixtures and integration regressions for tate-chu-yoko, PairPos filtering/cursor/ValueRecord semantics, HVAR/gvar metric selection, and composite gvar. The committed composite-gvar oracle was generated independently with fontTools 4.68.0. Normal tests need neither fontTools nor installed fonts; verification instructions and fixture checksums are in `doc/testing.md` and `tests/fixtures/manifest.toml`.
- Tate-chu-yoko is explicit, single-face, single-line, and LTR only. It supports current-color, clip-free filled paths and fits advance width without clipping overhanging ink. Automatic digit grouping, BiDi, multi-font fallback, and multiline composition are not implemented.
- Point-attached composites retain the legacy unvaried-outline fallback; correct point matching is not implemented. `USE_MY_METRICS` inherited phantom positions/hinting are absent: its own phantom deltas are ignored while supported XY outlines vary, with `hmtx`/HVAR and `vmtx`/VVAR metric fallback. Simple-glyph rounding and the public integer outline representation are unchanged; arbitrary rotated/sheared composites are not validated.
- GPOS mark attachment keeps its existing separate filtering behavior. Device/VariationIndex adjustments and broad real-font/HarfBuzz parity remain unimplemented or unverified, respectively. See `doc/feature-status.md` for the detailed support boundaries.

## 0.0.16 - 2026-11-11

### Compatibility

- Update the published `bin-rs` dependency to 0.1.0. Callers passing `bin-rs` readers to low-level APIs must also use `bin-rs` 0.1.0; reader types from `bin-rs` 0.0.11, used by fontcore 0.0.15, are not interchangeable.
- Preserve the high-level APIs, legacy raw API names, default and optional features, and Rust 1.91 minimum. This release changes dependency metadata only; font parsing and shaping code is unchanged.

## 0.0.15 - 2026-11-10

### Compatibility and fixes

- Update the published `bin-rs` dependency to 0.0.11. Callers passing `bin-rs` readers to low-level APIs must also use `bin-rs` 0.0.11; reader types from `bin-rs` 0.0.9, used by published fontcore 0.0.14, are not interchangeable.
- Preserve name-table UTF-16 byte-length handling with the updated reader API, including rejection of odd byte lengths.
- Read the fixed-size OS/2 PANOSE and vendor-ID fields with the updated reader API’s exact-read method.
- Add a UTF-16 name-table regression test. The Rust 1.91 minimum and public feature set are unchanged.

## 0.0.14 - 2026-11-10

### Compatibility and licensing

- Require Rust 1.91 or later, aligned with WML2. CI verifies the minimum version on native and WebAssembly targets.
- Include pinned Unicode data under Unicode-3.0. The package license expression is now `MIT AND Unicode-3.0`; the Rust implementation remains MIT licensed.
- Preserve existing raw API enum names. Header and glyph string formatting now uses `Display` and the standard `ToString` implementation.

### Added

- GPOS SinglePos adjustments and explicit sideways-LTR and mixed vertical-LTR shaping, measurement, and SVG APIs.
- A public vertical-orientation query using pinned Unicode 18.0.0 data.
- Extended grapheme boundaries using pinned Unicode 17.0.0 data and unicode-segmentation 1.13.3, with complete upstream boundary test data.

### Fixed

- Vertical flow without font metrics, vertical origins for TrueType, CFF, CFF2, and static COLR v0, and application of vertical variation metrics.
- Legacy vertical GSUB feature selection, SingleSubst format 1 glyph deltas, lookup ordering, and chaining.
- GPOS mark and pair Y-coordinate handling, plus ZWJ/control and grapheme-aware layout behavior.
- Name-table language-tag offsets and UTF-16 decoding, including rejection of odd byte lengths.

### Verification

- Make normal tests self-contained with committed synthetic font fixtures; keep external font-corpus tests opt-in.
- Add regression coverage for the changes above and preserve the documented limitations in `doc/feature-status.md` and `doc/implementation-status.md`.
