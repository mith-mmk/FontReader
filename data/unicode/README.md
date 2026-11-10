# Unicode data and versions

fontcore's `vertical_orientation(char)` query is pinned to Unicode **18.0.0**.
It exposes the scalar property only. This data does not implement grapheme
segmentation, font substitution, or automatic mixed-orientation layout.

The internal shaping/fallback text boundaries use Unicode **17.0.0** extended
grapheme clusters, separately pinned as described below. Neither version states
that fontcore implements complete Unicode shaping, BiDi, or CSS mixed layout.

## Enclosing marks (General_Category=Me)

The internal `unicode::is_enclosing_mark(char)` query independently pins
`General_Category=Me` to **Unicode 18.0.0**. This category is not equivalent to
all combining marks, grapheme extenders, or a segmentation library's data.

- Official source: <https://www.unicode.org/Public/18.0.0/ucd/extracted/DerivedGeneralCategory.txt>
- Local original: [`18.0.0/DerivedGeneralCategory.txt`](18.0.0/DerivedGeneralCategory.txt)
- Source header date: 2026-06-29, 15:25:02 GMT
- Retrieved: 2026-11-09
- Source size: 282,143 bytes
- Source SHA-256: `d6b151d2d40ee9b1876d26f417980f45ffae47b6055ccf7203cb31f07a030f94`
- License: **Unicode-3.0**, the same unchanged [Unicode License V3](LICENSE.txt)
  and license hash documented below. The original and derived table are
  copyright (c) 2026 Unicode, Inc.

The generated table contains 13 scalars in five inclusive ranges:
`0488..0489`, `1ABE`, `20DD..20E0`, `20E2..20E4`, and `A670..A672`.
The allocation-free internal lookup uses binary search. Build and runtime do
not need Python, network access, or source parsing.

```console
python tools/generate_enclosing_marks.py
python tools/generate_enclosing_marks.py --check
cargo test --offline --no-default-features --lib enclosing_mark
```

The deterministic offline generator verifies source size, source/license hashes,
version header, valid scalar ranges, ordering, overlap, and range/scalar counts.
The independent Rust test parses every category in the original file, verifies
complete nonoverlapping code-space coverage, and compares every one of the
1,112,064 valid scalars with the lookup. Additional tests pin all five ranges,
their boundaries, and non-Me combining marks. The originals are kept byte-for-byte
by `.gitattributes`; generated Rust uses LF. Review source, license, hashes,
counts, tests, and behavior together when updating this Unicode pin.

内部の囲み文字判定も Unicode 18.0.0 の `General_Category=Me` に固定しています。
結合文字全般や書記素分割の判定ではありません。原本・ライセンスのハッシュを
検証して 13 文字・5 範囲を生成し、独立した Rust テストで全スカラー値を照合します。

## Provenance and license

- Official source: <https://www.unicode.org/Public/18.0.0/ucd/VerticalOrientation.txt>
- Local original: [`18.0.0/VerticalOrientation.txt`](18.0.0/VerticalOrientation.txt)
- Source header date: 2026-06-29, 15:25:28 GMT
- Retrieved: 2026-11-09
- Source size: 192,064 bytes; 2,499 explicit property rows
- Source SHA-256: `0803e09669d7aa7137de678e55b848bd418c6632694fa1b05f0b40941103c748`
- Property specification: [UAX #50, Unicode Vertical Text Layout](https://www.unicode.org/reports/tr50/)
- Database format: [UAX #44](https://www.unicode.org/reports/tr44/)
- License: **Unicode-3.0**, [Unicode License V3](LICENSE.txt), downloaded unchanged
  from <https://www.unicode.org/license.txt> on 2026-11-09
- License SHA-256: `e7a93b009565cfce55919a381437ac4db883e9da2126fa28b91d12732bc53d96`

The Vertical_Orientation data and derived Rust property table are copyright (c) 2026
Unicode, Inc., and covered by the accompanying Unicode license, not the project's
MIT license alone. The remaining project code is MIT-licensed. Cargo declares
the combined package as `MIT AND Unicode-3.0`. Both licenses, this provenance
file, the original data, and the generator are included in the crate package.

## Regeneration and verification

From the repository root, with Python 3 (standard library only):

```console
python tools/generate_vertical_orientation.py
python tools/generate_vertical_orientation.py --check
cargo test --offline --no-default-features --test unicode_vertical_orientation
```

Generation never downloads data. It checks the pinned source/license SHA-256 and version,
the single `@missing: 0000..10FFFF; R` default, sorted nonoverlapping source ranges,
property values, and the expected row and generated-range counts. It then omits
`R` ranges and merges adjacent ranges with the same remaining property, preserving
explicit unassigned and private-use ranges. The output has 192 sorted, inclusive,
non-`R` ranges. No source files are parsed and no allocation occurs at runtime;
the query performs a binary search over the compiled table.

The independent Rust test parser expands the original source and compares every
valid scalar and every source-range boundary with the public query. Among the
1,112,064 Unicode scalar values, the counts are `U = 332,511`, `R = 779,339`,
`Tu = 163`, and `Tr = 51`. Surrogates are explicitly `R` in the source, but cannot
be passed as Rust `char`; they are excluded from the scalar counts. Tests also
cover unassigned values, private use, noncharacters, supplementary planes,
variation selectors, combining marks, and characters introduced in Unicode 18.
Tests and generation require no network. Python is not needed to build the crate.

To update Unicode in the future, review the new official data and license, update
the version/hash/count pins and tests deliberately, regenerate, and document the
semantic changes. A Unicode version update can change returned values even for
previously unassigned scalar values. Do not substitute a different version's
property data or infer this property's version from a segmentation library.

## 日本語

`vertical_orientation(char)` は **Unicode 18.0.0** のスカラー値単位の
`Vertical_Orientation` を返します。書記素分割や字形置換、自動的な混在縦書きの
レイアウトは行いません。`Tr` は正立の縦書き用字形を優先し、その字形がない場合に
のみ回転します。単純な回転フラグに置き換えることはできません。

上記の公式 URL から取得した原本を変更せず同梱しています。Unicode データと
派生した Rust テーブルには同梱の **Unicode-3.0** ライセンスが適用されます。
プロジェクト本体の MIT ライセンスのみが適用されるわけではありません。

上記の Python コマンドで、ネットワークや追加ライブラリなしで再生成・照合できます。
生成時は原本・ライセンスの SHA-256、バージョン、既定値、範囲の順序・重複を検証します。既定値 `R`
以外の隣接範囲をまとめた 192 範囲を使用し、実行時にデータ解析やメモリ確保は
行いません。未割当文字・私用文字を含む全 1,112,064 スカラー値と各境界を、
独立した Rust の原本パーサーで照合します。サロゲートは `char` にできないため
スカラー値の件数から除きます。ビルドに Python やネットワークは不要です。

## Extended grapheme clusters: Unicode 17.0.0

The internal boundary iterator uses `unicode-segmentation = "=1.13.3"`, whose
`UNICODE_VERSION` is `(17, 0, 0)`. The exact Cargo.toml pin protects downstream
library users even when they do not use this repository's Cargo.lock. The crate
is MIT OR Apache-2.0, has no runtime dependencies, and declares Rust 1.85.0 as
its minimum version. fontcore has no declared overall MSRV; this dependency
floor is not a claim that the complete crate builds with Rust 1.85.0.

- Dependency: <https://crates.io/crates/unicode-segmentation/1.13.3>
- Implementation: <https://github.com/unicode-rs/unicode-segmentation>
- Rules: [UAX #29 revision 47 (Unicode 17)](https://www.unicode.org/reports/tr29/tr29-47.html)
- Official test source: <https://www.unicode.org/Public/17.0.0/ucd/auxiliary/GraphemeBreakTest.txt>
- Local original: [`17.0.0/GraphemeBreakTest.txt`](17.0.0/GraphemeBreakTest.txt)
- Source header date: 2025-03-24, 14:45:55 GMT
- Retrieved: 2026-11-09
- Size: 126,570 bytes; all 766 official test cases
- Source SHA-256: `e2d134d2c52919bace503ebb6a551c1855fe1a1faec18478c78fff254a1793ec`

The unchanged test data is copyright (c) 2025 Unicode, Inc., under the bundled
[Unicode-3.0 license](LICENSE.txt), whose source and checksum are above. The
fixture, license, test parser, and verification script are included in the crate
package. `.gitattributes` preserves original downloaded bytes on every platform.
No Unicode 18 grapheme segmentation or Unicode 18 mixed-layout conformance is
claimed. Updating this dependency or its Unicode version requires an explicit
review independently of the vertical-orientation property data.

```console
python tools/check_grapheme_data.py
cargo test --offline --no-default-features --lib unicode::graphemes
cargo test --offline --no-default-features --lib fontreader::text_control_tests
cargo test --offline --no-default-features --test unicode_grapheme_fallback
```

The Rust conformance test parses every official boundary marker independently
and compares forward and reverse extended grapheme iteration before any layout
or orphan-selector policy is applied. Adapter and public fallback tests run
separately. All verification is offline; Python and fontTools are not needed to
build or test the crate. Regenerating the synthetic MIT fallback font requires
fontTools 4.65.0 via `python tests/fixtures/generate_unicode_grapheme_fallback.py`.

### 拡張書記素クラスタのバージョン

内部のシェーピング／フォールバック境界には、`unicode-segmentation = "=1.13.3"`
による **Unicode 17.0.0** の拡張書記素クラスタを使用します。公開の
`vertical_orientation(char)` は別に **Unicode 18.0.0** へ固定されています。
Unicode 18 の書記素分割や、完全なシェーピング・BiDi・CSS 混植レイアウトへの
適合を示すものではありません。依存 crate の最低 Rust バージョンは 1.85.0
ですが、fontcore 全体の MSRV を宣言するものではありません。

公式の全 766 テストを省略せず同梱し、分割処理とレイアウト用の調整を別々に
検証します。上記のスクリプトで原本とライセンスの SHA-256 をオフライン確認
できます。Unicode の原本には同梱の Unicode-3.0 ライセンスが適用されます。
