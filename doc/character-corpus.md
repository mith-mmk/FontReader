# IPA and ISO-8859-derived Unicode probe (2026-11-16)

This is an **opt-in diagnostic**, not a production shaping change, a legacy
encoding decoder, a conformance test, or a claim of complete IPA/Hebrew support.
It is based on PR #59 (`c669218`), which depends on #58 and #57. Arabic was
already tested separately and is intentionally excluded from this new sweep.

## Reproduce

Requires stable Rust, Python 3, fontTools and the native HarfBuzz library.
The tool downloads nothing and does not install or redistribute fonts.

```sh
cargo build --example corpus_geometry
python -m unittest discover -s tools -p test_character_corpus.py
python tools/compare_character_corpus.py \
  --font /path/to/NotoSans-Regular.ttf \
  --probe target/debug/examples/corpus_geometry --output /tmp/noto-sans.json
```

Repeat with NotoSansHebrew-Regular.ttf. The committed reports in
`doc/character-corpus/` record font hashes, versions, missing coverage, every
input and both outputs. An error or incomplete probe output fails rather than
silently omitting a case. A comparison mismatch is diagnostic data, not a failed
process: not every difference means incorrect rendering.

## Corpus and reference limits

742 rows, generated deterministically from Python 3.12.14 standard-library
ISO-8859 codecs (parts 1–5, 7–11, 13–16), assigned U+0250–02FF characters, and 18
selected sequences in NFC/NFD forms. Every ISO scalar keeps its part/byte
provenance. Undefined bytes and control/format characters are excluded. Part 6
is deliberately excluded and part 12 does not exist. The separate Unicode
mapping tables were not downloaded or compared in this run; Python codec tables
are the actual input source. Unicode normalization/category data is Python's
15.0.0, not the library's distinct grapheme/vertical property versions.

The IPA sweep includes some modifier letters not specific to IPA and is **not**
an exhaustive IPA inventory; combining symbols are represented only in selected
sequences (rings, syllabicity, tilde, acute, tie bar, aspiration, length, stress).
The ISO-derived inventory includes Greek/Cyrillic/Thai, but unsupported cmap
scalars are recorded as missing, never counted as successful shaping.

Reference: system HarfBuzz **10.2.0**, OpenType font functions, no variations,
explicit LTR or RTL direction and locale, script guessed from each complete
string. This is not paragraph BiDi, fallback, font-family selection, vertical,
GUI, Chrome or raster/pixel comparison. The older Hangul reports used HB 15;
these results must not be described as that version.

Fontcore size is hhea ascent - descent + lineGap, so its public positions are in
font design units. Both engines use a baseline at y=0, inverted to screen y.
Fontcore RTL's right-edge origin is translated by the total advance to compare
HarfBuzz's left-edge origin. Per-glyph advances and final ink bounding boxes are
compared as an unordered multiset, since public RTL vector order differs.
Tolerance is 0.1 units for advances and 0.51 for bounds (HarfBuzz rounds composite
extents to integers; fontcore retains fractional outlines). Glyph IDs, clusters,
contours and overlapping pixel coverage are not compared. Bounds equivalence
alone is weaker than full shape equivalence.

## Recorded results

| Font | cmap-covered rows | Per-glyph geometry matches | Aggregate ink bounds matches |
| --- | ---: | ---: | ---: |
| Noto Sans Regular | 620 / 742 | 612 | 617 |
| Noto Sans Hebrew Regular | 36 / 742 | 36 | 36 |

All eight Noto Sans structural differences are decomposed NFD sequences.
Five ordinary accents (é, å, š, ő, ü) use separate glyphs where HarfBuzz composes;
the aggregate bounds match. That is **not proof of an error**, nor proof of
identical contours. Three cases also have differing aggregate bounds:

- `a + tilde + acute`: top -961 vs HarfBuzz -965 (4 design units).
- `a + ring below + tilde`: bottom 227 vs HarfBuzz 273 (46 units).
- `a + breve + acute`: top -967 vs HarfBuzz -844 (123 units).

These are reproducible geometry discrepancies requiring normalization/mark
ownership investigation, not yet fixed. The selected Hebrew words and marks
match after origin normalization, but no complete BiDi claim is made. NFC/NFD
rows that are identical after normalization remain separate labelled controls;
row counts are not counts of distinct strings or independent failures.

References: https://www.unicode.org/Public/MAPPINGS/ISO8859/ ;
https://github.com/harfbuzz/harfbuzz ;
https://github.com/notofonts/noto-fonts .

## Checks

Python harness unit tests: 3 passed. Rust all-features: 547 passed / 1 ignored;
no-default: 244 passed; default: 429 passed / 1 ignored. Strict all-target,
all-feature Clippy and formatting passed on Rust 1.100.0. Independent review
found no blockers; coverage and geometry limitations above were clarified.
This is a local result, not a claim about the eventual PR CI or MSRV run.
