# Implementation status

This file is the implementation status for the next `fontcore` release. A
feature is only marked complete when it has a specification-focused test and a
real-font regression where applicable.

## Verification

- Normal CI does not require the untracked `.test_fonts` corpus; the external
  corpus module is compiled only when `FONTCORE_TEST_FONTS` is set.
- Corpus tests use `FONTCORE_TEST_FONTS` when it is set; for example:
  `FONTCORE_TEST_FONTS=/path/to/fonts cargo test --all-features --lib`.
- Synthetic parser tests remain part of the normal Rust test suites.
- HarfBuzz is a development-time comparison tool, not a runtime dependency.

## Current scope

| Area | Status | Evidence |
| --- | --- | --- |
| sfnt / TTC loading | partial | bounded input and table-range checks |
| WOFF loading | partial | length, range, decompression, checksum checks |
| WOFF2 loading | partial | declared input length and decoded-size checks |
| `cmap` 4 / 12 / 14 | partial | synthetic format and UVS tests |
| GSUB / GPOS | partial | lookup and locale regression tests |
| GDEF | partial | relative-base and Offset32 parser fixes |
| CPAL / COLR v0 | partial | palette-index and current-color tests |
| OpenType SVG | partial | bounded gzip and conservative syntax allowlist |
| measurement | partial | measurement is derived from the shaped run |
| CFF2 | implementation present, regression pending | synthetic charstring tests |

The status table intentionally distinguishes parsing, execution, and real-font
regression coverage. It is not a replacement for the test names or the
corresponding OpenType specification sections.
