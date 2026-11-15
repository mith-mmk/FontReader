# Testing

The normal test suite uses the committed MIT synthetic fonts in
`tests/fixtures/`. No installed fonts, downloaded font corpus, or fontTools
installation is needed. The fixture manifest records their generators and
checksums. The HTTP loading test serves a committed fixture on loopback; it
does not download a font from the internet.

Run the CI checks with the current stable Rust toolchain:

```sh
cargo test --no-default-features
cargo test
cargo test --features "layout,cff"
cargo test --all-features
cargo check --all-features
cargo check --examples --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

The all-feature test job runs library, integration and documentation tests on
both stable Rust and the minimum supported version. Do not restrict this job to
`--lib`: integration tests gated by `cff2`, `svg-fonts` or `raw` are not enabled
by the default-feature jobs. Compiling them with Clippy or `cargo check` does
not execute their assertions.

## External font corpus

Real-font compatibility tests are compiled when `FONTCORE_TEST_FONTS` is set.
`build.rs` tracks this variable and enables `fontcore_external_corpus`; it is
independent of Cargo feature selection. The emoji SVG test fails when its
required font is missing; it does not silently skip. Other corpus tests may
select an available alternative font or return when optional fonts are absent.

For the emoji SVG smoke test, obtain the static COLRv1
`NotoColorEmoji-Regular.ttf` from the
[Noto emoji project](https://github.com/googlefonts/noto-emoji) and place it at
the root of your corpus directory. Use the font's original license. Run only
this corpus test with:

```sh
FONTCORE_TEST_FONTS=/absolute/path/to/fonts cargo test --all-features --lib emoji_font_renders_svg
```

PowerShell equivalent:

```powershell
$env:FONTCORE_TEST_FONTS = 'C:\path\to\fonts'
cargo test --all-features --lib emoji_font_renders_svg
Remove-Item Env:FONTCORE_TEST_FONTS
```

To run the full external corpus suite, supply the other fonts named by the
helpers in `src/test.rs` (including `ZenMaruGothic-Regular.ttf` and
`NotoSansJP-Regular.otf`), then run `cargo test --all-features --lib` with the
same variable. Some families have alternative filenames selected by the
helpers. Unset the variable to return to the self-contained CI suite.


### Contextual GSUB regressions (unreleased)

`tests/contextual_gsub.rs` consumes the 13-face, original MIT
`contextual-gsub.ttc`. Regenerate with fontTools 4.68.0 using
`python tests/fixtures/generate_contextual_gsub.py`; ordinary Rust tests need
neither Python nor installed fonts. The first five regressions fail against
0.0.17/main and pass with sequence shaping enabled. Both default and
`--no-default-features` behavior are asserted. Existing vertical lookup-chain
faces 8/15/17 now explicitly assert complete Multiple output in text APIs;
raw scalar tests continue to assert no flattened result.

Unit regressions also cover format 1 records, nested lookup bounds, no-op rules,
reverse Extension traversal, deletion, output cursor movement, ligature counts,
and conservative GDEF filtering. This suite is not a claim of general script
shaping, mark-skipping, or full real-font/HarfBuzz equivalence.

### Basic Tibetan vowel positioning (unreleased)

`tests/tibetan_vowel_positioning.rs` uses an original MIT collection generated
by `python tests/fixtures/generate_tibetan_vowel_positioning.py` with
fontTools 4.68.0. Ordinary tests require neither Python nor an installed font.
The paired control faces omit GPOS while retaining all other tables; they check unchanged out-of-scope and layout-disabled output.
Run `cargo test --test tibetan_vowel_positioning` and repeat with
`--no-default-features` and `--all-features`.

The earlier anchor-only stage was checked with official Noto Serif Tibetan 2.103, weight 400
(SHA-256 `060ec022b04c306de3f58d051fb0e1cf81a5b610c5910fbfc43bba154c057cda`).
At fontcore size 2815 (unit font scale for this face), U+0F40 U+0F72 now places
the vowel at (90, 0), rather than (0, 0), matching HarfBuzz 15 full shaping.
U+0F40 U+0F74 places the separate vowel at (60, 525), matching only the
HarfBuzz diagnostic with `blws=0`; full HarfBuzz substitutes a single glyph.
Do not interpret that diagnostic comparison as full Tibetan shaping parity.
A local Chrome comparison at 100%/200% confirmed the above-vowel placements
for ka+i/e/o/reversed-i and the below-vowel GPOS-only movement for ka+u.
Ka+aa remains a separate, incorrect glyph with a visible dotted-circle form in
this font; including a vowel in the fallback whitelist does not guarantee a
usable anchor or replace missing `blws` substitutions. Stacks, subjoined
letters, multiple marks and orphan vowels remain outside the repair.
Screen Y coordinates are the negation of HarfBuzz Y-up positions. The external
font, license and raw diagnostic output are not part of the repository.

### Bounded Tibetan stack substitutions (unreleased)

`tests/tibetan_stack_substitution.rs` uses 30 original MIT faces plus paired
controls generated by `python tests/fixtures/generate_tibetan_stack_substitution.py`
with fontTools 4.68.0. The generator needs no external input and fixes timestamps.
Normal Rust tests need neither Python nor installed fonts. Tests cover explicit
`tibt`/LangSys/required-feature selection, stage and lookup ordering, deduplication,
whole-run context, exact filtering and flag precedence, Extension/nested mutation,
rollback of incomplete or ambiguous sources, missing-glyph family fallback,
unsupported mixed runs, actual orientation features, scale and no-layout behavior.
A parser unit test checks the optional trailing mark-filtering set and every
truncated prefix. Run the suite with default, all and no-default features.

An independent local comparison used the same Noto Serif Tibetan 2.103 font,
weight 400, and uharfbuzz 0.57.0 / HarfBuzz 15.0.0 with explicit OT shaping,
Tibetan script, LTR direction and default features. Across 30 codepoint probes
and five locales (150 cases), 35 changed outputs matched full HarfBuzz glyph
counts, advances, origins and ink bounds; the other 115 retained identical
fontcore metrics and SVG. Improvements included ka+aa, ka+u, ka+subjoined-ka,
sa+subjoined-ga+subjoined-ra+u, ga+subjoined-ha, ka+subjoined-ssa and tsa+i.
The separate full-HarfBuzz and feature-disabled diagnostics were not conflated.
Stack+upper-vowel, compound/precomposed, compatibility, orphan and mixed runs
remain limitations. Even two individually supported stacks can roll back when
a needed lookup excludes a neighboring mark. See the feature-status document
for the deliberately narrow whole-run acceptance rules.

Repeated ka+subjoined-ka runs of 20/200/2000 clusters were approximately
5.6/54/540 ms in a local debug build (previous stage: 2.2/20/206 ms). This is a
near-linear, roughly 2.6x cost on that workload, not a general performance claim.
External fonts, local logs and visual artifacts are not redistributed.

### Bounded Tibetan canonical normalization (unreleased)

`tests/tibetan_normalization.rs` uses eight original MIT synthetic faces with
paired no-presentation controls. Regenerate with fontTools 4.68.0 using
`python tests/fixtures/generate_tibetan_normalization.py`; fixed timestamps make
the result byte-identical (checksum in the fixture manifest). The Rust suites
need no external fonts, Python or network. Tests cover the 13 potentially admitted
canonical mappings, lower-vowel order reversal, five locales, repeated runs,
selector/compatibility/upper-vowel exclusions, missing components, missing final
glyphs, expansion/filtering rollback, and original-grapheme family selection.
The stack suite tests normalized GHA through distinct vertical-flow, mixed and
sideways orientation alternatives. Private tests check original source exports,
original scalar spans versus normalized ligature component counts, reordered
provenance, and early rejection of 100,000 alternating marks.

```console
python tools/generate_tibetan_normalization.py --check
cargo test --offline --lib tibetan
cargo test --offline --test tibetan_normalization
cargo test --offline --test tibetan_stack_substitution
cargo test --offline --no-default-features --test tibetan_normalization
```

The official Unicode 17 original, license, source hash, all 17 canonical mappings
and 19 nonzero CCC values are checked independently as described in
[data provenance](../data/unicode/README.md#tibetan-canonical-properties-unicode-1700).
Those data counts are not a promise that all 17 forms enter the shaping plan.
The previous stack-only experiment above is historical; the current acceptance
limits are in the feature-status documents. No general normalization, modified
Tibetan CCC, upper-vowel stack positioning or punctuation itemization is claimed.

An independent local comparison used official Noto Serif Tibetan 2.103, weight
400 (SHA-256 `060ec022b04c306de3f58d051fb0e1cf81a5b610c5910fbfc43bba154c057cda`),
and uharfbuzz 0.57.0 / HarfBuzz 15.0.0, explicit OpenType shaping, Tibetan script,
LTR direction and default features. Across 77 literal codepoint patterns and
five locales (385 configurations), all 41 changed outputs matched full-HarfBuzz
glyph counts, total advances, individual origins and ink bounds within 1e-5 px;
344 configurations retained byte-identical SVG and identical metrics to the
preceding bounded-stack stage. Among 85 canonical pairs (17 mappings times five
locales), fontcore equivalence gaps fell from 45 to 30; all 85 pairs agreed in
HarfBuzz. The remaining gaps are documented limitations, not passing support.
A separate 28-case Arabic nonregression set retained identical SVG and metrics
against the same preceding stage; this does not validate Arabic parity.

Browser visual inspection at 100%/200% covered UU/composed/decomposed/reordered
forms, GHA/KSSA and deferred II/stack+I/tsheg cases, including reset and reload.
It inspected the generated SVG comparison rather than claiming browser-native
shaping parity. Independent full-HarfBuzz output was kept separate from earlier
feature-disabled diagnostics. Repeated UU at 20/200/2000 clusters took about
6/59/588 ms in a local debug build versus 3.5/26/258 ms before this stage: an
observed near-linear workload, roughly 2.3x at the largest size, not a general
performance claim. External fonts, local logs and visual reports are not
redistributed in the repository.


### Bounded modern Hangul composition (unreleased)

`tests/hangul_composition.rs` uses the original MIT, 11-face
`hangul-composition.ttc`. Regenerate with fontTools 4.68.0 using
`python tests/fixtures/generate_hangul_composition.py`; normal Rust tests
require neither Python nor installed fonts. Timestamp-fixed fixture bytes and
the generator are tracked with a manifest checksum.

Unit tests enumerate all 11,172 modern syllables and canonical LV/LVT/LV+T
forms. Synthetic font tests cover composed-only cmap, both forms, missing
composed glyphs, no partial-LV substitution, missing original jamo, exact
internal GIDs, original source characters and per-EGC spans, one nominal
component after composition, GSUB/context/orientation/variant stages, mixed
Latin, tabs/newlines, repeated syllables, non-unit size, LTR/RTL/vertical
routes, whole-grapheme family fallback and layout-disabled controls.
Out-of-scope old/extended jamo, fillers, repeated/compatibility jamo, tones,
selectors, joiners, marks and Prepend characters remain nominal.

A failed GSUB candidate is tested directly and after family run grouping,
including a composed-only neighboring syllable and an unrelated pre-existing
missing glyph. In-memory truncation of the original MIT CFF and CFF2 fixtures
checks that an out-of-range target is rejected before outline access while
valid original jamo remain renderable. No malformed external font is needed.
A noncomposed-run predicate regression checks that the additional coverage
probe is not entered for Latin, emoji, Tibetan or already-composed Hangul.

Run the focused tests with default, all and no-default features:

```sh
cargo test --offline --lib hangul
cargo test --offline --test hangul_composition
cargo test --offline --all-features --lib hangul
cargo test --offline --all-features --test hangul_composition
cargo test --offline --no-default-features --test hangul_composition
```

The algorithm coverage is not an exhaustive font/shaping oracle. Generic
GSUB source-span and contextual-family fallback policies remain unchanged;
restoration requires nominal cmap coverage and does not cover custom GSUB
repair of missing original cmap entries. This stage does not claim Old Hangul,
full Korean, AAT or browser parity.
