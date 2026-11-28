# FontReader / fontcore 実装状況

更新日: 2026-11-28
対象ブランチ: `main`
基準コミット: `b02da19` (`fontcore 0.0.22`)
直近リリースコミット: `b02da19` (`fontcore 0.0.22`)

## このファイルの使い方

このファイルは、現時点の実装状態と次の作業だけを管理する。過去のレビュー指摘や完了済み作業を別の未完了項目として重複記載しない。

- `[x]` 実装と回帰テストが完了
- `[~]` 部分実装、既知の制限、または追加検証が必要
- `[ ]` 未実装または延期
- 仕様判定は OpenType 1.9.1、WOFF File Format 1.0、UAX #9 / #14 / #29、UAX #50 を基準にする
- 完了判定の根拠はテスト名、対応規格、`doc/implementation-status.md`、`doc/feature-status.md`、fixture manifest とする

## 実装済み

### 読み込み・安全性・CI 基盤

- [x] `FontFile` / `FontFace` の公開読み込み入口と `*_with_limits`
- [x] `DecodeLimits` による入力、表、WOFF、WOFF2、SVG 展開・参照量の制限
- [x] `bin-rs 0.1.0` への移行と sfnt / TTC の表境界検証、空 TTC・不正 face 選択の拒否
- [x] WOFF の宣言長、表範囲・重複、圧縮後長、checksum、metadata/private data 検証
- [x] Extension parser 検証および layout table 境界検証
- [x] 外部入力由来の確認済み panic 経路（TTC、CPAL、GSUB/GPOS index 等）の修正
- [x] CI 整備・緑化（`cargo fmt --all -- --check`、`cargo clippy --all-targets --all-features -- -D warnings`、no-default / default / layout+cff / all-features、examples、doc、MSRV 1.91、`wasm32-unknown-unknown`、Unicode/Grapheme 生成データチェック）
- [x] `FONTCORE_TEST_FONTS` 指定時だけ外部フォント corpus を有効化する方針と合成 fixture manifest（`tests/fixtures/manifest.toml`）

### OpenType 表・可変フォント・整形

- [x] `cmap` Format 4 / 12 / 13 / 14 の `idDelta`、補助平面、既定・非既定 UVS、および非対応 UVS/IVS の基底文字フォールバック
- [x] `fvar` / `avar` の境界・hidden flag・軸数・順序・必須 map 点の検証、`HVAR` / `VVAR` / `MVAR` メトリクス変動
- [x] `gvar` の composite variation（matched-point 合成および `USE_MY_METRICS` phantom 継承）
- [x] GDEF の `AttachList`、`MarkGlyphSetsDef`、`ItemVariationStore`、可変フォント対応 `CaretVariationStore`（Static Format 1 / bounded Format 2 / unadjusted Format 3）
- [x] GDEF による一般 GSUB / GPOS ルックアップフィルタリング（`IgnoreMarks` / `MarkAttachmentType` / `MarkFilteringSet`）
- [x] GSUB Context / Chaining の `SequenceLookupRecord` 保持と Format 1–3 の適用、Multiple 置換上限・自己参照防止、Reverse Chaining、スカラー `locl` 合字
- [x] GPOS Pair / Mark / `mkmk`（unanchored mark 原点・先行字検索修正）、Type 3 Cursive Attachment（`curs`）、`VariationIndex` デルタ、Vertical `SinglePos`
- [x] standalone default-ignorable グリフの出力抑制
- [x] CPAL v0（パレット相対 index・`0xFFFF` CurrentColor）、COLR v0、sbix の既存実装と vertical origin 対応
- [x] CFF2 charstring の `vsindex` / `blend` と Private DICT の既存実装
- [x] OpenType SVG の gzip ISIZE / CRC、構文 allowlist、危険要素・属性・外部参照拒否、自己完結型抽出、および型付きレイアウトコンテキスト（`SvgGlyphRun` / `SvgGlyphLayer`）

### Unicode・スクリプト対応

- [x] Unicode 17.0.0 拡張書記素クラスタ分割とクラスタ単位フォールバック
- [x] Unicode 18.0.0 `Vertical_Orientation`（`U` / `R` / `Tu` / `Tr`）の静的テーブルと公開 API（`vertical_orientation`）
- [x] 限定正規化・合成（現代ハングル、旧ハングル / 声調、かな濁点・半濁点、ラテン合成、チベット語正規化・積層・母音配置）
- [x] アラビア語コンテキスト連結（joining）と `curs` 接続

### 上位レイアウト・縦書き・キャレット

- [x] `ShapedText` によるソース位置保持付き整形（mapped shaping）
- [x] `CaretLayout` / `hit_test` / `visual_carets` / GDEF 合字キャレット・`CaretSource` / `CaretQuality`
- [x] `HardLineLayout`（LF / CRLF 改行の複数行整形と行間キャレット）
- [x] `WrappedLineLayout`（貪欲ソフトラップ、`WordOrGrapheme` / `Grapheme` / `JapaneseKinsoku` / `JapaneseKinsokuHanging`、`FontFamily` / `CompositeFont` 複数行、行境界 `Upstream` / `Downstream` affinity）
- [x] `BidiParagraphLayout`（UAX #9 + `unicode-bidi-mirroring` による段落双方向配置、isolate 境界保護、joining 破断検出）
- [x] `VerticalMappedLayout`（縦書き混植と座標変換済みキャレット）、`FontFamily::shape_vertical_mixed_ltr_mapped`、`VerticalWrappedLayout`（複数列折り返し・禁則・縦中横統合）
- [x] `FontEngine::shape_tate_chu_yoko_ltr` による明示的縦中横コンポジション
- [x] `AnnotatedLineLayout` によるルビ（振り仮名）・傍点（圏点）レイアウト API（縦横両対応、Center / Start / SpaceAround 揃え）
- [x] `CompositeFont` による `UnicodeRange` ルールベースの合成フォント

## 部分実装・既知の制限

- [~] 表単位の境界検証は進んでいるが、全表を統一 bounded reader / `TableProvider` へ移行する監査は未完了
- [~] 公開 API 全体の count・offset・index・確保量に対する網羅的ファジング／resource-limit 監査
- [~] `WrappedLineLayout` は JIS X 4051 / JLREQ 基本の禁則処理（追い出し・句読点ぶら下げ）に対応済み。完全な UAX #14 行分割および揃え位置（alignment / justification）は未対応
- [~] `BidiParagraphLayout` は hard line のみ対応で、ソフトラップおよび `FontFamily` フォールバックと未統合
- [~] COLR v1 / CPAL v1 が未実装（現在は v0 のみ）
- [~] `FeatureVariations` 実行時置換、任意 OpenType feature 指定、`FontVariant::SmallCaps` の `smcp` / `c2sc` 結線が未完了
- [~] CFF2 の再配布可能な真の可変フォント fixture、軸別 outline 署名、malformed corpus 回帰が未完了
- [~] `doc/feature-status*` に比べて `doc/implementation-status.md` の同期が遅れている

## 次の実装項目（0.0.23+ ロードマップ）

### M1: 日本語組版・縦書き実用化（`noveleditor` GPUI 連携向け）

- [x] `WrappedLineLayout` への禁則処理（行頭禁則・行末禁則・分離禁止・句読点ぶら下げ：`JapaneseKinsoku` / `JapaneseKinsokuHanging`）
- [x] `VerticalMappedLayout` の拡張（`FontFamily::shape_vertical_mixed_ltr_mapped`、複数列折り返し `VerticalWrappedLayout`、縦中横スパンのインライン統合）
- [x] ルビ（振り仮名）・傍点（圏点）レイアウト API（`AnnotatedLineLayout`：縦横両対応、親文字スパン紐付け、折り返し連携）

受入条件: 日本語小説の縦書き・横書き段落（ルビ・傍点・縦中横・禁則・行長折り返しを含む）を単一または合成フォントでレイアウトし、キャレット位置・ヒットテスト不変条件および `unified/` 視覚回帰を通過すること。

### M2: 段落レイアウト・BiDi・複数行の統合

- [ ] `BidiParagraphLayout` へのソフトラップ統合と `FontFamily` フォールバック対応
- [ ] 段落アライメント（左・中央・右・両端揃え）、タブ展開、段落字下げ / インデントの設計・実装

受入条件: 混在 LTR / RTL 段落の複数行折り返しとアライメントで視覚順序・joining・`hit_test` 往復が崩れないこと。

### M3: OpenType カラーフォント・シェイピング拡張

- [ ] COLR v1 / CPAL v1 の実装とグラデーション・合成・可変レイヤーの `GlyphLayer` / bounds 対応
- [ ] `FeatureVariations` 実行時置換、任意 OpenType feature 指定、`FontVariant::SmallCaps`（`smcp` / `c2sc`）結線
- [ ] CFF2 再配布可能可変フォント fixture、軸別 outline 署名、malformed corpus 回帰の整備

受入条件: COLR v1・`FeatureVariations`・CFF2 可変軸が合成/実フォント fixture で検証され、CI マトリクスを通過すること。

### M4: 境界検証監査・ファジング・保守

- [ ] 全テーブル loader の統一 bounded reader / `TableProvider` 監査完了と `DecodeLimits` 全展開・確保経路接続
- [ ] `from_buffer`、face 列挙、metadata、`shape`、`measure`、`render_svg` を対象としたファジングの追加
- [ ] `unified/` クロスライブラリ結合スイート（`fontcore` + `paintcore` + `wml2`）の継続運用と `doc/implementation-status.md` 同期

受入条件: 任意バイト列で panic せず、上限超過が明示的な resource-limit error で停止すること。

## 保留する項目

- [ ] TrueType hinting（`cvt `、`fpgm`、`prep`、`gasp`）
- [ ] BASE / JSTF / MATH
- [ ] EBDT / EBLC / EBSC、CBDT / CBLC
- [ ] DSIG / PCLT / VDMX / LTSH / MERG
- [ ] 複雑な SVG pattern / filter の完全描画
- [ ] rich text 全般、OS 固有のインストール済みフォント探索

## 未実装・限定対応の言語 / スクリプト一覧

現行の `locale_to_script_tags` / `locale_to_language_system_tags`（`src/opentype/extentions/gsub.rs` 等）および `src/fontreader/` のスクリプト別整形実装を調査した結果、未実装または限定対応にとどまる言語・スクリプトは以下の通り。

### 南アジア系（Indic / USE — 未実装）

- [ ] デーヴァナーガリー（`deva` / `dev2`: ヒンディー語、マラーティー語、ネパール語、サンスクリット語）— 音節分割・`rphf`/`half`/`pstf`/`blwf` 再配置未実装
- [ ] ベンガル語（`beng` / `bng2`）— 二分割母音・結合子音処理未実装
- [ ] グルムキー文字（`guru` / `gur2`: パンジャーブ語）
- [ ] グジャラーティー文字（`gujr` / `gjr2`）
- [ ] オリヤー文字（`orya` / `ory2`）
- [ ] タミル語（`taml` / `tml2`）
- [ ] テルグ語（`telu` / `tel2`）
- [ ] カンナダ語（`knda` / `knd2`）
- [ ] マラヤーラム文字（`mlym` / `mlm2`）
- [ ] シンハラ文字（`sinh`）

### 東南アジア系（未実装）

- [ ] タイ語（`thai`）— 上下母音/声調配置・Sara Am (`U+0E33`) 分解・辞書ベース単語分割未実装
- [ ] ラーオ語（`lao `）
- [ ] クメール語（`khmr`）— Coeng 結合・`pref`/`blwf`/`abvf`/`pstf` 未実装
- [ ] ミャンマー語（`mymr` / `mym2`）— Kinzi・Medial・USE 再配置未実装
- [ ] ジャワ文字（`java`）・バリ文字（`bali`）・フィリピン系文字（`tglg` 等）

### 東アジア・中央アジアの未対応 / 限定対応

- [ ] モンゴル文字（`mong`）— 縦書き専用接続・FVS1–3 / MVS 異体字選択・joining 未実装
- [ ] 中国語ロケール変種マッピング（`ZHS` / `ZHT` / `ZHH`）および注音符号（`bopo`）
- [ ] アイヌ語仮名（`U+31F0–31FF`、セ+309A 等小書き仮名+半濁点の専用配置・`locl` 連携未対応）
- [ ] チベット語（`tibt`）— 現在は限定プランのみ。汎用スクリプト分割・複合母音（`U+0F73`/`U+0F81` 等）・`bo`/`dz` ロケール自動マッピング未対応

### 欧州・コーカサス・アフリカ・その他

- [ ] ギリシャ語（`grek`）・キリル文字（`cyrl`: `BGR`/`SRB` 等の `locl` マッピングおよびスクリプトタグ自動対応）
- [ ] アルメニア語（`armn`）・ジョージア語（`geor`: `mtvr` 等含む）
- [ ] エチオピア文字（`ethi`）
- [ ] ンコ文字（`nko `）・アドラム文字（`adlm`）・ターナ文字（`thaa`: RTL/joining 汎用スクリプト分割未実装）
- [ ] チェロキー文字（`cher`）・カナダ先住民音節文字（`cans`）

## 検証コマンド

必須テスト（外部フォント・ネットワーク不要）:

```text
cargo test --all-features --lib
```

外部 corpus（明示指定時のみ）:

```text
FONTCORE_TEST_FONTS=<path> cargo test --all-features --lib
```

補助・CI 検証:

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --no-default-features
cargo test --no-default-features --features "layout,cff"
cargo test --all-features
cargo check --examples --all-features
cargo check --target wasm32-unknown-unknown --all-features
cargo doc --all-features --no-deps
```

直近の検証結果（2026-11-28 / `fontcore 0.0.22` `b02da19` 時点）:

- `cargo test --all-features --lib`（サンドボックス / オフライン環境）: 380 テスト中 375 passed、4 ignored、1 failed（`test::tests::fontload_from_net_works` がオフライン環境のソケット拒否 os error 10013 により失敗）
- 統合テスト（`tests/*.rs`）: 52 ファイル配置済み（今回未実行）
