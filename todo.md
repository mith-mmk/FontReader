# FontReader / fontcore 実装状況

更新日: 2026-09-02
対象ブランチ: `main`
基準コミット: `819f677a2363f9e21b71b77a1f5bcf3d2f31b600`
直近リリースコミット: `0784157` (`fontcore 0.0.13`)

## このファイルの使い方

このファイルは、現時点の実装状態と次の作業だけを管理する。過去のレビュー指摘や完了済み作業を別の未完了項目として重複記載しない。

- `[x]` 実装と回帰テストが完了
- `[~]` 部分実装、既知の制限、または追加検証が必要
- `[ ]` 未実装または延期
- 仕様判定はOpenType 1.9.1、WOFF File Format 1.0、UAX #9 / #29を基準にする
- 完了判定の根拠はテスト名、対応規格、`doc/implementation-status.md`、fixture manifestとする

## 実装済み

### 読み込みと安全性

- [x] `FontFile` / `FontFace`の公開読み込み入口と`*_with_limits`
- [x] `DecodeLimits`による入力、表、WOFF、WOFF2、SVG展開量の制限
- [x] sfnt / TTCの表境界検証と空TTC・不正face選択の拒否
- [x] WOFFの宣言長、表範囲・重複、圧縮後長、checksum、metadata/private data検証
- [x] 外部入力由来の今回確認済みpanic経路（TTC、CPAL、GSUB/GPOS indexなど）の修正

### OpenType表と整形の既存範囲

- [x] `cmap` Format 4 / 12 / 13 / 14のidDelta、補助平面、既定・非既定UVS
- [x] `fvar` / `avar`の境界、hidden flag、軸数、順序、必須map点の検証
- [x] GDEFのAttachList、MarkGlyphSetsDef、ItemVariationStoreに関する相対基点・Offset32修正
- [x] GSUB Context / ChainingのSequenceLookupRecord保持とFormat 1–3の既存適用経路
- [x] GSUB Multiple置換の出力上限、自己参照防止、Reverse Chainingの右から左適用
- [x] GPOS pair / mark系の既存適用と同一lookup内の重複サブテーブル適用防止
- [x] GSUB / GPOSのlocale、script、required feature、無効indexの安全な処理
- [x] CPAL v0のパレット相対indexと`0xFFFF` CurrentColor、COLR v0
- [x] CFF2 charstringの`vsindex` / `blend`とPrivate DICTの既存実装
- [x] OpenType SVGのgzip ISIZE / CRC、構文allowlist、危険要素・属性・外部参照の拒否
- [x] `shape()`由来の既存測定経路、font size / stretch回帰

### 検証基盤

- [x] 通常push / pull request向けのtest、all-features、examples、doc、WASM check CI
- [x] `FONTCORE_TEST_FONTS`指定時だけ外部フォントcorpusを有効化
- [x] 合成fixture manifest（`tests/fixtures/manifest.toml`）
- [x] HarfBuzzを開発時の差分比較器として利用する方針

## 部分実装・既知の制限

- [~] 表単位の境界検証は進んでいるが、全表を統一`TableProvider` / bounded readerへ移行する監査は未完了
- [~] GSUB / GPOSの全Type 1–8 / 1–9共通実行器、lookup flag、MarkFilteringSet、FeatureVariations実行時置換
- [~] GDEFのglyph class・mark filteringの実行時結線とmark-to-ligatureのcomponent選択
- [~] GSUB Multipleの入力cursor・固定点反復・巨大出力に対するファジング／包括的回帰
- [~] 公開API全体のcount・offset・index・確保量に対するpanic／resource-limit監査
- [~] Reverse Chainingの複数実フォント差分検証
- [~] CFF2の再配布可能な真の可変フォントfixture、軸別outline署名、malformed corpus回帰
- [~] `shape()`、`measure()`、`render_svg()`が詳細な同一`GlyphRun`／レイアウト結果を共有するAPI統一
- [~] CIのstrict fmt / Clippyは既存のリポジトリ全体のformat・lint負債により未緑化

## 次の実装項目

### M1: 境界検証の共通化

- [ ] 共通bounded reader / `TableProvider`へ全loaderを移行する
- [ ] すべてのcount、offset、加算、乗算、slice取得を共通境界APIへ集約する
- [ ] `DecodeLimits`を全展開・確保経路へ接続する
- [ ] `from_buffer`、face列挙、metadata、shape、measure、render_svg、raw dumpを対象にpanic・無限ループ・NaN/Infinityのファジングを追加する

受入条件: 任意バイト列でpanicせず、上限超過は明示的なresource-limit errorで停止する。

### M2: GSUB / GPOS共通実行器

- [ ] `TextRun`、`GlyphInfo`、`GlyphPosition`、`GlyphBuffer`を導入する
- [ ] lookup index順、lookup flag、MarkFilteringSet、GDEF classを共通実行器で扱う
- [ ] GSUB Type 1–8とGPOS Type 1–9、Extension、Device / VariationIndexを統合する
- [ ] Context / Chainingの指定`sequenceIndex`へ入れ子lookupを適用し、置換後もclusterを維持する
- [ ] 同一lookupの適用回数・置換長に上限を設け、無条件の固定点反復を行わない

受入条件: 合字・複数置換・文脈置換・位置調整のglyph ID、cluster、advance、offsetをfixtureで検証できる。

### M3: Unicode itemizationとfallback

- [ ] UAX #29拡張書記素cluster分割を整形入力へ導入する
- [ ] UAX #9の段落、埋め込みlevel、視覚順を導入する
- [ ] script / language / direction / face単位の`TextRun`分割を行う
- [ ] Latin、日本語、Arabic、Syriac、Hebrew、Tibetan、Emoji/VS/ZWJのfixtureを整備する
- [ ] `FontFamily`と家族列を分離し、`FontFallbackList` / `LastResortFace`を追加する
- [ ] Unicode caseless matchingとcluster単位fallbackを実装する
- [ ] `FontVariant::SmallCaps`を`smcp` / `c2sc`へ接続し、任意feature指定を追加する

受入条件: 混在LTR / RTL、VS/ZWJ、mark clusterでface分割やcluster分断が発生しない。

### M4: 測定・出力・色フォント

- [ ] `shape()`、`measure()`、`render_svg()`を同一の詳細整形結果から生成する
- [ ] logical advance、logical bounds、ink bounds、line boundsを分離する
- [ ] size、stretch、variationの適用を一度に統一する
- [ ] SVGを安全化済みsceneへ変換し、ID、transform、clip、gradient、mask座標を統一する
- [ ] 実レイヤーからboundsを算出し、COLR v0 / SVG / sbixで描画範囲を一致させる
- [ ] COLR v1 / CPAL v1を段階実装する

受入条件: shape末尾cursorとmeasureが一致し、サイズ倍・stretch倍・色レイヤーboundsの回帰が通る。

### M5: 整理と回帰拡充

- [ ] sfnt / WOFF / WOFF2の表dispatchを共通化する
- [ ] `cff2` featureを実コードとCI検証へ結線する
- [ ] 実フォント回帰とHarfBuzz差分比較を定期検査へ移行する
- [ ] `doc/feature-status*`の「パース済み」「実行可能」「実フォント回帰済み」を現行コードへ同期する
- [ ] strict fmt / Clippyの既存負債を段階的に解消する

## 保留する項目

整形コア完成後に扱う。現在の残実装の完了条件には含めない。

- [ ] TrueType hinting（`cvt `、`fpgm`、`prep`、`gasp`）
- [ ] BASE / JSTF / MATH
- [ ] EBDT / EBLC / EBSC、CBDT / CBLC
- [ ] DSIG / PCLT / VDMX / LTSH / MERG
- [ ] 複雑なSVG pattern / filterの完全描画
- [ ] rich text、ruby、OS固有のインストール済みフォント探索

## 検証コマンド

必須テスト（外部フォント不要）:

```text
cargo test --all-features --lib
```

外部corpus（明示指定時のみ）:

```text
FONTCORE_TEST_FONTS=<path> cargo test --all-features --lib
```

補助検証:

```text
cargo test --no-default-features
cargo test --no-default-features --features "layout,cff"
cargo check --examples --all-features
cargo check --target wasm32-unknown-unknown --all-features
cargo doc --all-features --no-deps
```

直近の検証結果（2026-09-02）:

- clean相当 all-feature: 108 passed
- 外部corpus: 240 passed、3 ignored
- no-default: 32 passed
- layout/cff: 39 passed
- examples、WASM check、doc生成: 成功
- strict fmt / Clippy: 既存のformat・lint負債により未成功
