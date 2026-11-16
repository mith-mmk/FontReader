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

## Fallback and cursor responsibilities (follow-up)

`FontFamily` (`src/fontface.rs`) owns family fallback and whole-cluster face
selection. `CompositeFont` (`src/compositefont.rs`) owns explicitly ordered,
rule-local fallback. A single-face `FontEngine` does not discover another font.
The caller must provide the faces; this library does not discover system fonts
or choose an operating-system fallback family.

The public `GlyphRun`/`PositionedGlyph` types (`src/commands.rs`) expose glyphs,
positions and metrics, but no source-cluster mapping, editor caret stops,
hit-testing or ligature-internal caret API. The library owns the shaping pen
position and following-glyph advances. Editor caret/navigation/selection policy
belongs to the consumer and cannot be verified by treating glyph indices as
Unicode indices. `measure(prefix)` is not a general caret solution: cutting a
string can change ligatures, kerning and combining context. Internal source
spans do not make a public caret mapping available.

Five self-contained tests in `tests/european_fallback.rs` use an original MIT
three-face fixture (generator/hash in the fixture manifest). They cover:

- Whole-cluster family fallback for all six Umlauts in NFC/NFD and ß/ẞ.
- An incomplete primary plus mark-only fallback must not split the base/mark
  cluster across faces; the documented first-face missing-glyph path remains.
- Composite primary → fallback → primary spans retain the following pen origin.
- NFC/NFD repeated Umlauts preserve pen advances despite scalar-count changes.
- A synthetic `fi` ligature uses its shaped advance for following glyphs, with
  explicit layout-disabled control behavior.

These are fontcore-owned contracts, not real-font fallback/GUI or consumer caret
validation. No consumer repository or caret implementation was changed, no
arbitrary liga/dlig switch was added, and no new production API is assumed.

Follow-up validation: the five new tests pass with default and no-default
features. The complete all-feature suite passes 562 tests (one ignored).
Strict all-target/all-feature Clippy, formatting and fixture byte-identical
regeneration pass on Rust 1.100.0. No PR/CI run is claimed for this branch.
