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


### Contextual GSUB regressions (0.0.18)

`tests/contextual_gsub.rs` consumes the 13-face, original MIT
`contextual-gsub.ttc`. Regenerate with fontTools 4.68.0 using
`python tests/fixtures/generate_contextual_gsub.py`; ordinary Rust tests need
neither Python nor installed fonts. The first five regressions fail against
the 0.0.17 baseline and pass with sequence shaping enabled. Both default and
`--no-default-features` behavior are asserted. Existing vertical lookup-chain
faces 8/15/17 now explicitly assert complete Multiple output in text APIs;
raw scalar tests continue to assert no flattened result.

Unit regressions also cover format 1 records, nested lookup bounds, no-op rules,
reverse Extension traversal, deletion, output cursor movement, ligature counts,
and conservative GDEF filtering. This suite is not a claim of general script
shaping, mark-skipping, or full real-font/HarfBuzz equivalence.

### Basic Tibetan vowel positioning (0.0.18)

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

### Bounded Tibetan stack substitutions (0.0.18)

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

### Bounded Tibetan canonical normalization (0.0.18)

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


### Bounded modern Hangul composition (0.0.18)

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

### Bounded canonical kana composition (0.0.18)

The original MIT `kana-composition.ttc` generator covers all 58 Unicode17 pairs,
including voiced iteration marks, with both-form/composed-only/original-only,
invalid-target, orientation/context GSUB, fallback, and mixed Hangul rollback
faces. Its marks deliberately have nonzero advance; Ainu named sequences are
unchanged rather than hidden by a generic zero-advance workaround.
Independent unit tests parse pinned UnicodeData and CompositionExclusions,
prove table completeness, starters/nonrecursive bases/exclusion safety, reject
extra marks/selectors/joiners/prepends/halfwidth input, and check source spans
and CFF/CFF2 bounds. Existing Hangul and Tibetan tests exercise the shared guard.
Integration checks cover direct/family shaping, six locales, horizontal/RTL/
vertical/mixed/sideways layouts, composed-only coverage, whole-EGC fallback,
missing targets, invalid GSUB in both mixed-script orders, composed-only
neighbors, raw scalar access and no-layout behavior.

This synthetic suite does not establish full shaping parity for Japanese, Ainu,
Korean or Tibetan. Real-font comparison results are recorded separately when
available; no GUI or independent comparison pass is implied by these tests.

仮名58組と合成除外の独立検証、元文字・範囲、CFF/CFF2 の境界、両順序の
ハングル混在復元、縦横・混在方向、フォールバック、raw／layout 無効時を検査します。
アイヌ語 named sequence の送りは既存値を保ちます。これらは言語全般の shaping
一致や GUI 検証の完了を意味しません。

Independent real-font checks on 2026-11-15 used Noto Sans CJK JP Regular 2.004
(SHA-256 `68a3fc98800b2a27b371f2fb79991daf3633bd89309d4ffaa6946fd587f375b5`)
and locally installed Hiragino Sans W3 face 0, 22.0d2e1
(`ff92033a300f55aff2f45f57479a560af2c5467973b425d3f724f870af07f83e`, not redistributed).
The reference was uharfbuzz 0.57.0 / HarfBuzz 15.0.0, explicit OpenType shaper,
Kana script, upem 1000, LTR/TTB with Y-down coordinate conversion; locale probes
were absent/ain/ja/jpn/en (HB und/ain/ja/ja/en). Across two fonts, five locales
and horizontal/vertical/mixed modes, all 1,740 isolated canonical pairs matched
NFC geometry (baseline: zero); comparable LTR/TTB counts, advances, pen origins
and ink matched HarfBuzz exactly. Glyph-ID parity was not assessed; mixed
vertical was a fontcore NFC/NFD comparison without a HarfBuzz oracle. All 630 Ainu core cases and 6,570 no-layout outputs
were unchanged. Separate prior controls retained 376 Hangul, 385 Tibetan,
28 Arabic and eight mixed cases, including 797 byte-identical SVGs.

Extra-mark, selector/joiner, Prepend and halfwidth controls were unchanged.
Pure-kana words became canonical-equal, but nonligated vertical mark placement
and some vertical punctuation still differ from HarfBuzz. Hiragino mixed runs
containing unsupported Hangul can restore otherwise valid kana through the
conservative whole-run rollback described above. Whole-buffer Kana-script
HarfBuzz probes are not itemized mixed-script reference shaping.
Generated SVGs were visually inspected in Chrome at 100% and 200%, including
kana marks/iteration marks, Ainu named sequences and mixed words; selection,
zoom and reload were checked. This tests fontcore SVG output, not Chrome's
native text shaping, and does not imply all broader shaping issues are solved.

独立実フォント検証では上記2フォントの単独58組×5 locale×3方向、計1,740件が
NFC と等価になり、アイヌ630件と layout 無効6,570件は不変でした。既存の
ハングル・チベット・アラビア文字等797 SVG も不変です。Chrome で生成 SVG を
100/200%表示し代表例を目視確認しました。これは Chrome 自体の組版ではありません。
未合字の縦書き記号、一部の縦約物、未対応ハングルを含む Hiragino の混植 run
による保守的復元は残る制限であり、全面的な shaping 一致は主張しません。

## Variation-selector regressions

`cargo test --test unsupported_ivs --all-features` runs 12 synthetic regressions;
`--no-default-features` runs the six layout-independent controls. Adding `raw` to the no-default run
adds a seventh control for the public raw UVS glyph/SVG wrappers. The original
MIT fixtures and generator are recorded in `tests/fixtures/manifest.toml` and
regenerate byte-for-byte with fontTools 4.68.0. Coverage includes absent format
14/selector/pair, exact default/nondefault mappings, supplementary selectors and
bases, missing bases, repeated/orphan selectors, multi-pair EGCs, horizontal/
vertical/mixed rendering, exact versus nominal family choice, continuity,
explicit glyph-zero contextual repair, invalid nominal/post-GSUB mark IDs, and
CFF/CFF2 invalid CharStrings with color metadata. The cmap unit test also checks
inclusive default ranges and preserves historical low-level return behavior.

Raw UVS glyph/SVG rendering now retains unsupported bases; public signatures
are unchanged. These tests do not claim original scalar source indices, a
registered-sequence filter, full CSS fallback, or universal Chrome parity.
Font-authored unregistered mappings can differ from Chrome policy. Separate
local real-font/Chrome checks are diagnostic, not required for the offline suite.

The composite raw getter regression in `tests/composite_gvar.rs` runs with
`--no-default-features --features raw` and with `raw,layout` (also included by
`--all-features`). Both configurations check horizontal metrics against the
independent fixture oracle. With `layout`, it also checks vertical metrics;
without `layout`, it asserts `None` because this TTC's `vhea`/`vmtx` tables are
not loaded. The test does not change runtime behavior or extend that assertion
to other font containers.

## Bounded composition beside unsupported neighbors

`kana_composition` compares exact public glyph geometry for kana and modern
Hangul beside untouched missing glyphs, including variation selectors,
composed-only coverage, direct/family routes and horizontal, vertical, mixed
and sideways rendering. Original synthetic fixture faces 10–13 retain the
conservative rollback for context-only damage, non-first composed ligature
inputs, expansion to a missing output and identity substitution of a missing
neighbor. Internal tests additionally cover identity locale/joining operations
and ensure a touched Tibetan identity is not a presentation change. This stage
does not expose public source clusters or localize contextual causal effects.

## Bounded residual kana advances

`cargo test --test kana_residual` uses 22 original MIT fixture faces for GDEF
absence versus absent/populated class definitions, effective SinglePos/PairPos
ownership including zero records, GSUB provenance, missing glyphs, exact EGC
admission, directions, no-layout behavior and composition rollback. Library
tests check legacy raw export. `cargo test --all-features`, default,
`--no-default-features` and `--features raw` cover the relevant API routes.
The generator and SHA256 are in the fixture README/manifest. Licensed real-font
checks stay local. See [policy limits](kana-residual-marks.md); shrinking advance
does not guarantee good attachment or prevent following-glyph ink overlap.

## Bounded mark input filtering

- `cargo test --lib mark_input_filter`
- `cargo test --test mark_input_filter`
- `cargo test --no-default-features --test mark_input_filter`

The original 34-face MIT fixture covers types4/5/6 and ExtensionPos, input flags,
mark-filter precedence, missing GDEF/classes/sets, zero anchors, later eligible
lookups, locale controls and vertical/no-layout regressions. Regenerate with
`python tests/fixtures/generate_mark_input_filter.py` using FontTools4.68.0;
the fixture manifest pins its SHA256. See [scope and real-font controls](mark-input-filtering.md).
The explicit synthetic GDEF-origin fallback case is a remaining limitation,
not a full HarfBuzz output equality assertion.


### Bounded Tibetan upper-stack proof (0.0.18)

`tests/tibetan_upper_stack.rs` uses an original deterministic MIT 48-face TTC.
It covers six upper vowels, deeper stacks, extension anchors, ordinary mark
priority, an adversarial type-5 anchor on an explicit Base, canonical consonant
ownership, exact scale and vertical/rotated geometry, and transactional failures.
Controls remove only abvs/blws activation. Failure cases include missing GDEF or
classes, wrong classes/features, filtered/missing anchors, missing cmap/final
glyphs, unresolved subjoined marks, expansion/recombination, upper consumption,
and cross-cluster ligation. Punctuation, mixed scripts and unsupported vowel
sequences retain existing whole-run exclusions. Private tests assert exact
original source intervals and prevent generic Mkmk eligibility on these outputs.

The fixture is generated by `tests/fixtures/generate_tibetan_upper_stack.py` with
fontTools 4.68.0 and no external font. Tests require neither Python nor network.
This stage does not establish full Tibetan, AAT, or browser parity.

### SVG document extraction regressions

The SVG table unit tests cover document-list-relative offsets with a nonzero
list and table offset, shared documents referenced by disjoint glyph ranges,
encoded document bounds, and gzip output bounded independently of encoded
length. Shared encoded payloads use one allocation. Selection tests distinguish
`glyph1` from `glyph10` and from non-ID attributes, accept quoted ID whitespace,
and reject missing glyph IDs in a tagged shared document.

This remains bounded SVG extraction, not a general XML engine or complete SVG
rendering: ancestor styling/transforms and arbitrary shared references still
need the later SVG command-set work. The untagged single-document fallback is
retained for compatibility.
