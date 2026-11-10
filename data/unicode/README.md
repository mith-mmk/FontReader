# Unicode Vertical_Orientation data

fontcore's `vertical_orientation(char)` query is pinned to Unicode **18.0.0**.
It exposes the scalar property only. This data does not implement grapheme
segmentation, font substitution, or automatic mixed-orientation layout.

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

The Unicode data and derived Rust property table are copyright (c) 2026
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
