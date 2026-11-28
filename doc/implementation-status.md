# Implementation status

This file summarizes the current implementation status as of `fontcore 0.0.23` (`main` after M1–M4). A feature is only marked complete when it has a specification-focused test and a real-font regression where applicable. For detailed layout contracts and limitations, see [doc/feature-status.md](feature-status.md).

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
| sfnt / TTC loading | implemented | bounded input, `bin-rs 0.1.0` migration, unified `TableProvider` / `bounded_table_reader`, cumulative `DecodeLimits` checks, and `fuzz_and_limits` regression tests |
| WOFF / WOFF2 loading | partial | length, range, decompression, checksum, `WoffTableProvider` / `DecodeLimits` checks, and safe `Unsupported` fallback |
| `cmap` 4 / 12 / 13 / 14 | partial | synthetic format, UVS/IVS base fallback, bounds-hardened `try_get_subtable`, and `cmap` regression tests |
| GSUB / GPOS | partial | lookup, locale, GDEF filtering, `curs`, `VariationIndex`, `mkmk`, contextual, `FeatureVariations` (`GSUB`/`GPOS` v1.1), and custom `FontFeatureSetting` / `smcp` tests |
| GDEF | partial | relative-base, Offset32 fixes, lookup filtering, and variable `CaretVariationStore` |
| Variable fonts (`fvar`/`avar`/`gvar`/`HVAR`/`VVAR`/`MVAR`) | partial | composite matched-point variation, `USE_MY_METRICS`, and metrics variation tests |
| CPAL v0/v1 / COLR v0/v1 / sbix | partial | palette types/labels, current-color, vertical origin, and COLR v1 layers/gradients/transforms/clip tests |
| OpenType SVG | partial | bounded gzip, syntax allowlist, self-contained extraction, and typed `SvgGlyphRun` context |
| CFF2 | partial | synthetic charstring tests, `cff2-vertical-origin.ttc` variable outline/origin regressions, and truncated container guards |
| Measurement & carets | partial | `ShapedText`, `CaretLayout`, `hit_test`, `visual_carets`, and GDEF ligature caret tests |
| Line & paragraph layout | partial | `HardLineLayout`, `WrappedLineLayout` (Kinsoku / hanging / `ParagraphOptions`), `BidiParagraphLayout` (family / wrapped), `VerticalMappedLayout`, `VerticalWrappedLayout`, `tate_chu_yoko`, `CompositeFont` |
| Japanese advanced typesetting (Kinsoku / Ruby / Bouten) | partial | `WrapMode::JapaneseKinsoku` / `JapaneseKinsokuHanging`, `VerticalWrappedLayout`, `AnnotatedLineLayout` (`ruby` / `emphasis`) |

The status table intentionally distinguishes parsing, execution, and real-font regression coverage. It is not a replacement for the test names or the corresponding OpenType / Unicode specification sections.
