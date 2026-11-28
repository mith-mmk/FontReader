# FontReader / fontcore 実装状況

更新日: 2026-11-28
対象ブランチ: `main`
基準コミット: `1edf03d` (`fontcore 0.0.23`)
直近リリースコミット: `1edf03d` (`fontcore 0.0.23`)

## このファイルの使い方

このファイルは、現時点の実装状態と次の作業だけを管理する。過去のレビュー指摘や完了済み作業を別の未完了項目として重複記載しない。

- `[x]` 実装と回帰テストが完了
- `[~]` 部分実装、既知の制限、または追加検証が必要
- `[ ]` 未実装または延期
- 仕様判定は OpenType 1.9.1、WOFF File Format 1.0、UAX #9 / #14 / #24 / #29、UAX #50 を基準にする
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

- [~] `WrappedLineLayout` は JIS X 4051 / JLREQ 基本の禁則処理（追い出し・句読点ぶら下げ）と `ParagraphOptions`（揃え・インデント・タブ）に対応済み。完全な UAX #14 行分割・両端揃えの高度化は今後拡張対象
- [~] 公開 API のエラー型は `std::io::Error` をそのまま返しており、構造化 `FontError` への整理が未実施
- [~] `FontFile::face()` / `FontFace::clone()` および長文整形（5 万字規模）向けの `Arc` 共有・キャッシュが未整備
- [~] エディタ・IME 向けの選択範囲矩形（`selection_rects` / `range_bounds`）API が未整備
- [~] UAX #24 ベースの汎用スクリプト分割と南アジア/東南アジア等の一部スクリプトが未実装

## 次の実装項目（0.1.0 ロードマップ）

合意方針:
- 位置づけ: 現行機能の API 整理・安定化 ＋ 実アプリ連携（`paintcore` / `noveleditor` 向け完成度・性能向上）＋ UAX #24 汎用スクリプト分割を含む言語拡充
- 互換性: deprecated 項目削除・公開 enum への `#[non_exhaustive]` 付与・`#[non_exhaustive]` な `FontError` への統一（`raw` feature は opt-in 維持）
- リリース戦略: M1〜M4 完了後に **`0.1.0-rc.1`** を先行リリースして `paintcore` / `unified/` / `noveleditor` で実機検証し、フィードバック反映後に **`0.1.0` 正式版** を発行する

### 0.1-M1: 公開 API 整理と構造化 `FontError` 導入

- [ ] `FontError`（`#[non_exhaustive]` enum: `Io`, `ResourceLimit`, `InvalidFont`, `InvalidInput`, `Unsupported`, `NotFound` 等、`Display` / `std::error::Error` / `From<FontError> for std::io::Error`）を `src/error.rs` に新設し、公開 API の戻り値を `Result<T, FontError>` へ統一する
- [ ] deprecated 項目（`commads` エイリアス、`LoadedFont` 等）を削除し、公開 enum（`WrapMode`, `TextAlignment`, `GlyphPaint`, `FontVariant` 等）へ `#[non_exhaustive]` を付与する
- [ ] `CHANGELOG.md` または移行ノートに `0.0.23 → 0.1.0` の破壊的変更点を明記する

受入条件: `--all-features` / `--no-default-features` / `wasm32` / examples / doc が通り、`DecodeLimits` 超過や非有限入力が `FontError` バリアントで判別できること。

### 0.1-M2: `Arc` 共有化とグリフ／シェイピングキャッシュ（長文性能基盤）

- [ ] `FontFile` / `FontFace` 内部の `Font` データを `Arc` 共有化し、`file.face(i)` / `FontFace::clone()` を O(1) 化する
- [ ] スレッド安全・容量上限付きのグリフ解決 / メトリクス / アウトライン / シェイピングキャッシュ（可変座標キー対応）を導入する
- [ ] 5 万字規模の日本語段落（ルビ・禁則・縦書き折り返し含む）ベンチマーク／性能回帰テストを追加する

受入条件: キャッシュ有効時と無効時でレイアウト・描画結果が完全一致し、繰り返し整形・`face` 複製でディープコピーが発生しないこと。

### 0.1-M3: 選択範囲・IME 候補窓向け矩形 API（`selection_rects` / `range_bounds`）

- [ ] `TextRect` と `selection_rects(byte_range)` / `caret_rect(byte_offset)` を `HardLineLayout`、`WrappedLineLayout`、`BidiParagraphLayout`、`VerticalWrappedLayout`、`AnnotatedLineLayout` に実装する
- [ ] 行またぎ選択、BiDi 視覚順の不連続矩形分割、縦書き列またぎ選択、IME `bounds_for_range` の単一書記素矩形に対応する

受入条件: すべてのレイアウト API で `hit_test` ↔ `caret_rect` ↔ `selection_rects` の幾何不変条件テストが通過すること。

### 0.1-M4: UAX #24 汎用スクリプト分割と言語・ロケール拡充

- [ ] Unicode `Script` / `Script_Extensions` 静的テーブルと UAX #24 スクリプトラン分割（`Common` / `Inherited` 解決含む）を実装する
- [ ] ギリシャ語（`grek`）・キリル文字（`cyrl`: `BGR`/`SRB` `locl`）、中国語ロケール変種（`ZHS` / `ZHT` / `ZHH`）＋注音符号（`bopo`）、チベット語（`bo`/`dz` 自動マッピング）、アイヌ語仮名（`U+31F0–31FF`・小書き仮名+半濁点回帰）を整備する
- [ ] アルメニア語（`armn`）・ジョージア語（`geor`）・エチオピア文字（`ethi`）・ヘブライ文字（`hebr`）・シリア文字（`syrc`）の汎用スクリプト分割と合成 fixture 回帰を追加する

受入条件: 混在スクリプト文字列でスクリプト境界・`locl` 選択が崩れず、合成 fixture 回帰および Unicode 生成データチェックを通過すること。

### 0.1-M5: `fontcore 0.1.0-rc.1` リリース ＆ `paintcore` / `unified/` / `noveleditor` 実機検証

- [ ] `fontcore 0.1.0-rc.1` をリリース（タグ `v0.1.0-rc.1`）し、`todo.md` / `doc/implementation-status.md` / `CHANGELOG.md` を同期する
- [ ] `paintcore` 側を `fontcore 0.1.0-rc.1`（`FontError`, COLR v1 グラデーション/クリップ, `AnnotatedLineLayout` 描画）へ追従する
- [ ] `unified/` 視覚スイートに `0.0.23`〜`0.1.0` 新機能ケース（COLR v1・ルビ・禁則・選択矩形等）を追加して全件 PASS を確認する
- [ ] `noveleditor`（GPUI Phase 0）で IME `bounds_for_range`・縦書き閲覧・5 万字性能の実機ゲートを検証する

### 0.1-M6: RC フィードバック反映 → `fontcore 0.1.0` 正式リリース

- [ ] `0.1.0-rc.1` の実機検証で検出された API 微調整・不具合修正を反映する
- [ ] 全 CI マトリクス・MSRV・WASM・`unified/` 最終確認後に `fontcore 0.1.0` リリース PR マージおよびタグ `v0.1.0` を発行する

## 完了済みマイルストーン（`0.0.23`）

- [x] **M1** (PR #79): 日本語禁則・縦書き混植行分割（`VerticalWrappedLayout`）・ルビ／傍点（`AnnotatedLineLayout`）
- [x] **M2** (PR #80): 段落アライメント・インデント・タブ（`ParagraphOptions`）および `BidiParagraphLayout` のソフトラップ／`FontFamily` 統合
- [x] **M3** (PR #81): COLR v1 / CPAL v1、`FeatureVariations`・任意 feature 指定・`with_small_caps`、CFF2 可変アウトライン回帰
- [x] **M4** (PR #82): `TableProvider` / `bounded_table_reader` 統一、`DecodeLimits` 累計検証、`cmap`/`glyf` 境界強化、`fuzz_and_limits` ファジング追加

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

- [ ] **0.1.0 スコープ**: ギリシャ語（`grek`）・キリル文字（`cyrl`: `BGR`/`SRB` 等の `locl` マッピングおよびスクリプトタグ自動対応）
- [ ] **0.1.0 スコープ**: アルメニア語（`armn`）・ジョージア語（`geor`: `mtvr` 等含む）・エチオピア文字（`ethi`）・ヘブライ文字（`hebr`）・シリア文字（`syrc`）の UAX #24 汎用スクリプト分割
- [ ] **0.2.0+ へ繰越**: ンコ文字（`nko `）・アドラム文字（`adlm`）・ターナ文字（`thaa`）、チェロキー文字（`cher`）・カナダ先住民音節文字（`cans`）

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
