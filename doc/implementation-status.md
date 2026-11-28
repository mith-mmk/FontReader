# Implementation status

This file summarizes the current implementation status as of `fontcore 0.0.22` (`b02da19`). A feature is only marked complete when it has a specification-focused test and a real-font regression where applicable. For detailed layout contracts and limitations, see [doc/feature-status.md](feature-status.md).

## Verification

- Normal CI does not require the untracked `.test_fonts` corpus; the external corpus module is compiled only when `FONTCORE_TEST_FONTS` is set.
- Corpus tests use `FONTCORE_TEST_FONTS` when it is set; for example:
  `FONTCORE_TEST_FONTS=/path/to/fonts cargo test --all-features --lib`.
- Synthetic parser and layout tests remain part of the normal Rust test suites (380 unit tests in `src/` and 52 integration test suites in `tests/`).
- CI enforces `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, MSRV 1.91.0, `wasm32-unknown-unknown`, and Unicode/grapheme generated data verification.
- HarfBuzz is a development-time comparison tool, not a runtime dependency.

## Current scope

| Area | Status | Evidence |
| --- | --- | --- |
| sfnt / TTC loading | partial | bounded input, `bin-rs 0.1.0` migration, table-range checks; full table-provider audit pending |
| WOFF / WOFF2 loading | partial | length, range, decompression, checksum, and decoded-size checks |
| `cmap` 4 / 12 / 13 / 14 | partial | synthetic format, UVS/IVS base fallback, and `cmap` regression tests |
| GSUB / GPOS | partial | lookup, locale, GDEF filtering, `curs`, `VariationIndex`, `mkmk`, and contextual tests |
| GDEF | partial | relative-base, Offset32 fixes, lookup filtering, and variable `CaretVariationStore` |
| Variable fonts (`fvar`/`avar`/`gvar`/`HVAR`/`VVAR`/`MVAR`) | partial | composite matched-point variation, `USE_MY_METRICS`, and metrics variation tests |
| CPAL / COLR v0 / sbix | partial | palette-index, current-color, and vertical origin tests (COLR v1 / CPAL v1 not implemented) |
| OpenType SVG | partial | bounded gzip, syntax allowlist, self-contained extraction, and typed `SvgGlyphRun` context |
| CFF2 | implementation present, regression pending | synthetic charstring tests; redistributable variable fixture regression pending |
| Measurement & carets | partial | `ShapedText`, `CaretLayout`, `hit_test`, `visual_carets`, and GDEF ligature caret tests |
| Line & paragraph layout | partial | `HardLineLayout`, `WrappedLineLayout` (Kinsoku / hanging), `BidiParagraphLayout`, `VerticalMappedLayout`, `VerticalWrappedLayout` (`FontFamily` / TCY), `tate_chu_yoko`, `CompositeFont` |
| Japanese advanced typesetting (Kinsoku / Ruby / Bouten) | partial | `WrapMode::JapaneseKinsoku` / `JapaneseKinsokuHanging`, `VerticalWrappedLayout`, `AnnotatedLineLayout` (`ruby` / `emphasis`) |

The status table intentionally distinguishes parsing, execution, and real-font regression coverage. It is not a replacement for the test names or the corresponding OpenType / Unicode specification sections.
