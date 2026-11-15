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
