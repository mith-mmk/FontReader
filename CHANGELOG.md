# Changelog

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
