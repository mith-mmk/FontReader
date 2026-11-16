# Bounded European real-font comparison

Diagnostic extension on PR #62 (`1b519fabebb972f535d9a6965dee0b9bd5e090e9`).
No production code, public API, version, or dependency change; no merge/release.

## Reproduce

```sh
cargo build --example corpus_geometry
python -m unittest discover -s tools -p 'test_*corpus.py'
python tools/compare_character_corpus.py --corpus european \
  --font /path/to/NotoSans-Regular.ttf \
  --probe target/debug/examples/corpus_geometry --output /tmp/european.json
```

Requires Python, fontTools and native HarfBuzz. Nothing downloads fonts.
Repeat with the other two named fonts. Exact hashes/versions, every input and
raw geometry are in `european-corpus/*.json`. The original character corpus is
still the default and its construction is unchanged.

## Scope

55 rows per font: 14 ordinary default-feature ligature/context controls,
15 distinct-letter/word controls (including ß, ẞ, ss, SS, æ, œ), and 26
Umlaut rows (13 sequences in NFC and NFD). All six ä/ö/ü/Ä/Ö/Ü are present,
with words, repeated text, marks adjacent to ligatures and a ZWNJ control.
ß and ẞ are characters, not automatic ss substitutions. This does not test
case folding or uppercase conversion.

Standard horizontal LTR, explicit en/de locale, HarfBuzz 10.2.0; geometry
comparison/tolerances are identical to `character-corpus.md`. Public fontcore
positions/advances are compared, not glyph IDs, caret positions, contours,
pixels or GUI screenshots. NFC/NFD pairs are labelled controls, not independent
failures. No font family fallback or missing-glyph path was exercised: every
font's cmap covers every case.

The public engine does not expose arbitrary liga/dlig feature switches.
These cases test default features only. In particular, `st`/`ct` remaining
separate is not evidence of optional discretionary-ligature support. Existing
synthetic tests already cover ligature boundaries, mark ownership, contextual
GSUB, and scalar substitution guards; this complements rather than replaces them.

## Results

| Font | Covered | Per-glyph geometry match | Ink-union match |
|---|---:|---:|---:|
| Noto Sans Regular 2.004 | 55 | 55 | 55 |
| Noto Serif Regular 2.004 | 55 | 54 | 54 |
| DejaVu Sans 2.37 | 55 | 45 | 45 |

All six standalone Umlauts in NFC/NFD, standalone ß/ẞ, and plain fi/fl/ff/ffi/ffl
match in all three fonts. The mismatches remain recorded, not hidden:

- Noto Serif `f + ZWNJ + i`: same advance, but fontcore retains the font's visible
  ZWNJ outline whereas HarfBuzz makes it invisible. General default-ignorable
  handling is a separate production concern.
- DejaVu `f + diaeresis + i`: matching advances but different diaeresis bounds.
  Requires investigation of the font's combining-mark substitution/positioning;
  this is not a failure of the six canonically composed Umlaut letters.
- Nine DejaVu word/sequence rows differ in spacing: STRAẞE, STRASSE, groß,
  größer (NFC/NFD), Übergröße (NFC/NFD), ÄÖÜ äöü (NFC/NFD). Differences are 83, 81,
  45 or 36 units. The font's legacy kern R/A pair is -83, consistent with the
  first discrepancy, but that observation alone does not establish a complete
  root cause or a safe GPOS/legacy-kern fix. Do not change generic positioning
  based only on this diagnostic.

This checkpoint establishes bounded coverage and specific follow-up cases,
not complete European shaping parity. No speculative production fix was made.

## Validation

Six Python harness tests pass. Rust all-features: 557 passed / 1 ignored;
no-default-features: 247 passed. Independent read-only review found no blocker;
the spacing summary was corrected to include the 81-unit cases. These are local
checks on Rust 1.100.0, not a claim of CI/MSRV completion.
