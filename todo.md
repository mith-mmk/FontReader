# todo
- _test* が作業用フォルダ
- _test_fonts/* がテスト用フォント
```
- [+] 実装済み
- [x] 動作確認済み
- [*] 実装済みだが動作に不具合あり
- [-] 実装遅延
- [ ] 未処理、未確認タスク
```
- `todo.md`でタスク管理
- `issue.md`でイシュー管理
- `readme`と書いた場合はreadme.ja.mdが正本でread.mdが英語版である

# APIの大幅破壊的変更

　現在FontReaderのAPIに、責務が混在しています。
これを「使いやすい公開API」として再設計する。

　これに伴いバージョンをを0.0.4から0.0.10にアップデート

## 要件

### 1. レイヤ分離
- [x] FontFile（ファイル / TTC管理）
- [x] FontFace（1フォント単位）
  - [x] metadata
  - [x] to_stringはdumpに変更
- [x] FontEngine（shaping / rendering）
  - [x] text2glyph_run
  - [x] text2svg
  - [x] text2commands
  - [x] shaping
  - [x] gsub/gpos

これに伴い lib.rsなどに置いてあるコードを
  - [x] fontface.rs
  - [x] fontengine.rs
  - [x] fontfile.rs
に分散させる

### 2. API方針
- [x] フォーマット差（TTF/OTF/WOFF）を外に出さない( すべて metadata関数で取得)
  - [x] 必要な以外はpub(crate)にする
- [x] Optionやunwrapを公開APIに出さない
- [x] NameIDなど低レイヤは隠蔽
- [x] 低レイヤの情報は features=raw に移動

### 3. 必須API
- [x] face.family()
- [x] face.full_name()
- [x] face.weight()
- [x] face.is_italic()

- [x] engine.shape(text)
- [x] engine.measure(text)
- [x] engine.render_svg(text)

## 4. 制約
- 既存の内部構造はなるべく流用
- ゼロコピーを維持
- backward compatibilityは不要

## 5. 出力形式
- 最終的なRustコード（struct + impl）
- 変更理由の説明
- API設計の意図

## 対象コード
- lib.rs
- fontheader.rs
- fontreader.rs
- util.rs
- リファクタリングの影響が出るコード 

## examples
- [x] 新API変更に対応できるように新規examplesを作成する
- [x] 旧examplesも対応できるようにする(ただしfeatures=rawに分離)

# リファクタリング後のタスク
- [*] dead codeの削除隔離(woff2.rsなど)
- [*] examplesのテスト コードの修正ですむか --features rawがいるかいないか判定 `readme`にも反映 パス、ファイル名をハードコーディングしている部分は、引数に変える
- [x] readmeの整理。説明が技術資料すぎるので、APIとsample中心んしいてわかりやすく書き直す。 今の細かい仕様はdoc/の下に移動
- [x] github workflows(CI/CD)の作成 exampleのbuild(Windows x86/arm, Linux x64/arm, Mac x64/arm) タグがpushされたら起動
- [*] FontFamilyのフォールバック/GPOS/GDEF/GSUB適応順序の整理
- [ ] アラビア語フォントの対応 LTR RTLの責務は分離して持たせる
- [ ] スクリプト文字対応
- [ ] cff2対応を進める

- [+] web assemblyでもコンパイル出来るようにする
- [+] fontをbufferからloadする機能
- [*] commands.rsを利用し、pub fn text2commands(&text, FontOptions) -> Result<GlyphRun, Error>を実装
    - [+] TrueType, CFFは Pathに収納
    - [+] sbixはRasterに収納 // 実装済み。手元の sbix フォントは未所持なので自動テストは未追加
    - [+] svgは忘れる（取りあえずエラー）
    - [*] FontOptionsに必要なオプション
        - [*] FontFamiry, Font Name or Font // loaded Font 直渡しに加えて cache 済み FontFamily からの face 解決と glyph 単位 fallback まで実装。family chain / Last Resort は未実装
        - [+] font-size
        - [+] font-stretch
        - [+] font-style
        - [+] font-variant
        - [+] font-weight
        - [+] line-height
- [+] 上記を実現するのに不足している機能
- [ ] issueの処理と処理したissueを`issue.md`に追記
- [+] `todo.md`の更新
- [+] `README.ja.md`, `README.md`の更新

# TESTの実装(最優先)
- [+] font load from file
- [+] font load from net
- [+] font load form buffer
- [+] chunked font buffer
    - [x] WOFF2 を offset 付き chunk から再構成して load できる
- [+] font family cache
    - [x] `FontFamily` に loaded face を登録して weight/style/stretch で引ける
    - [x] `begin_chunked_face()` -> `append_chunk()` -> `finalize_chunked_face()` で chunked WOFF2 を cache に昇格できる
- [+] text to svg
- [+] text to command
- [+] text measure
- [+] lookup (すべてのパターン)
- [+] locale
    - [x] text2commands で実フォントの `locl` を確認
- [+] cmap (すべてのパターン)
- [+] 異字体セレクタ
    - [x] text2commands で format 14 の実データを 1 glyph cluster として扱う
- [+] emoji
- [+] 合字
    - [x] text2command / text2commands で基本合字(liga / dlig) を実データで確認
    - [ ] llga
    - [ ] 日本語  U+30D2（ヒ） + U+309A → ピ など
    - [ ] チベット語
    - [ ] 古ハングル
    - [ ] その他
- [+] 縦書き
    - [x] `FontOptions::with_vertical_flow()` で `text2commands` / `measure` / `FontFamily` を実フォント確認
- [+] 右から左に書く言語
    - [x] `FontOptions::with_right_to_left()` で Hebrew の RTL 配置を `text2commands` / `measure` / `FontFamily` で確認
    - [x] GSUB `isol` / `init` / `medi` / `fina` を使う Arabic joining を `text2commands` / `FontFamily` で確認
    - [x] GSUB `rlig` required ligature を `text2commands` / `FontFamily` で確認
    - [x] GSUB `rclt` / `calt` / `clig` を含む Arabic contextual shaping を `text2commands` / `FontFamily` で確認
    - [x] locale に応じて script (`arab` / `hebr` / `syrc` など) を優先し、required feature を含めて GSUB lookup を選ぶ
    - [*] context/chaining 依存の script 固有 shaping
        - [x] GSUB Context Format 1 / 2 / 3 の適用器
        - [x] GSUB Chaining Context Format 1 / 2 / 3 の適用器
        - [*] script 固有の chaining / language-specific lookup 拡張
            - [x] locale 全体 (`ur-Arab-PK` など) を見て language-specific lookup を選択
            - [ ] script ごとの実フォント chaining coverage をさらに拡張
- loader
    - [x] font
    - [x] font collection
- [+] woff
- [+] woff2
- [+] otf (CID-keyed CFF / FDSelect)
- [+] woffで以下のエラーが出るissue OS2 Headerのoutbound

# woff2の分割ファイル対策

複数のファイルが細切れに入って居る。lazy loadは側で実装側で対応するとして結合をchuck fontで行う
```html
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Fira+Sans:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&family=Noto+Sans+JP:wght@100..900&family=Noto+Sans:ital,wght@0,100..900;1,100..900&family=Roboto:ital,wght@0,100..900;1,100..900&display=swap" rel="stylesheet">
```

# Exampleの変更
- [+] ハードコーディングになっている部分を引数で渡せるようにする
- [+] txtを渡しているところは-s "string"で代替出来るようにする
  
# API
- [*] FontFamiry Class
  - [*] FontFamiryにフォールバックして探すシステム
    - [ ] 最下位にLast Resortが来る
    - [*] FontFamiryでfont weight, font style itaric, bold, normalを切り替えられる様にする
      - [x] cache 済み face から `font-weight`
      - [x] cache 済み face から `font-stretch`
      - [x] cache 済み face から `font-style`
      - [ ] font-variant
      - [*] cached faces 間の glyph fallback
      - [ ] family fallback chain
      - [ ] line-height を family default として保持
  - [x] `ChunkedFontBuffer` を使った face の取得途中状態を保持
  - [x] chunk 完了後に face を cache へ昇格
  - [x] `text2svg()` / `text2commands()` / `text2glyph_run()` / `measure()` / `options()` を `FontFamily` に追加
- [ ] Font Class
  - LoadedFontのラッパー
    - defalut font size, fontのフォールバック情報などのデフォルト情報を持つ
  - [+] font.text2svg(&self, &str, size: Option<f32>) -> String // textをsvgに変換して返す // textをsvgに変換して返す get_svgから再実装が必要
  - [+] font.text2command(&self, &str, size: Option<f32>) -> &FontCommand // textをコマンドにして返す
  - font.ritchtext2svg(&self, &FontText, size: Option<f32>) -> String // textをsvgに変換して返す 
  - font.ritchtext2command(&self, &FontText, size: Option<f32>) -> &FontCommand // textをコマンドにして返す

  - [+] font.measure(FontText) -> f32 // 長さ(px)
  - font.set_fontsize(f32)
  - font.get_fontsize() -> f32
  - font.set_line_spacing(f32)
  - font.get_line_spacing() -> 32


## APIの破壊的変更
- [+] load_font フォントロード from any
- [+] load_font_from_file フォントロード from file(exclude WASM)
- [+] load_font_from_buffer フォントロード from buffer
- [+] load_font_from_net フォントロード from NET(exclude WASM)
- [+] ChunkedFontBuffer による分割 buffer の再構成
- [+] `FontOptions::from_family()` / `with_family()` から `FontFamily` cache を利用
- [+] 重複していた旧 API に `#[deprecated]` を付与
    - [x] `fontload*` 系 alias
    - [x] `LoadedFont::text2command()` / `LoadedFont::text2commands()`
    - [x] `fontloader::commads`
 - [+] `full` feature から `encoding` を分離
    - [x] `full = ["layout", "cff"]`
    - [x] `encoding` は古い name table 互換として明示 opt-in に維持

# 合字対応
- [ ] llaga
- [*] アラビア文字
    - [x] `isol` / `init` / `medi` / `fina`
    - [x] `rlig`
    - [ ] context / chaining
    - [ ] 書き順の引き渡し
- [ ] 日本語
- [ ] チベット語
- [ ] 古ハングル
- [ ] 異字体セレクタ1 **確認中**
- [ ] 異字体セレクタ2 **確認中**
- [ ] 絵文字
# format
- [+] woff2対応
- [+] CID-keyed CFF / FDSelect
- [*] 境界条件をチェックしpanic!を回避
  - [x] optional raw table dump は欠損時に placeholder を返し panic しない
  - [x] `SourceSerif4-BlackIt.otf` の GSUB/GPOS `FeatureParams` 境界超過で panic しないよう修正
  - [x] GPOS 1.1 の壊れた optional `FeatureVariationList` は無視して続行する
  - [x] `hmtx` / `vmtx` の 0-metric edge case で panic しないよう修正
  - [x] lookup index out-of-bounds は panic ではなく `InvalidData` を返す
  - [ ] `opentype::mod` / `ttc` / `colr` など constructor 系の `unwrap()` をさらに減らす
- [x] svg svgのサイズが巨大なので文字毎にsvgを切り出す
# Layout 対応状況

`layout` feature は一部のみ実装されています。

- [*] layout featureの拡張
    - [x] text2command / text2commands / measure で variation selector と基本合字(liga / dlig) を利用
    - [x] `FontOptions::with_locale()` から `locl` shaping を利用
    - [x] `FontOptions::with_vertical_flow()` / `with_right_to_left()` を text API と `FontFamily` に反映
    - [*] ccmp / context chaining などの shaping 拡張
        - [x] `ccmp` の multiple / ligature / extension を text API shaping に反映
        - [x] RTL で `isol` / `init` / `medi` / `fina` を利用
        - [x] RTL で `rlig` を利用
        - [x] RTL で `rclt` / `calt` / `clig` を利用
        - [*] context / chaining の適用拡張
            - [x] Context Format 1 / 2 / 3
            - [x] Chaining Context Format 1 / 2 / 3
            - [*] script 固有の chaining / language-specific lookup 拡張
                - [x] locale 全体を見た language-specific lookup 選択
                - [ ] 実フォント coverage の拡張

# opentype
- [x] True Type
- [x] cff
- [ ] cff2
- [x] color true type
- [+] sbix
- [ ] svg  # svgパーサーがいる


# GPOS
- [*] 実装
- [x] pair adjustment (Format 1 / 2)
- [x] extension positioning (Type 9 経由の pair adjustment)
- [x] text2command / text2commands / measure への `kern` 反映
- [x] locale に応じて script を優先し、required feature を含めて `kern` lookup を選ぶ
- [ ] palt
- [ ] vpal
- [x] kern
- [ ] vkrn
- [ ] halt
- [ ] vhal

# GSUB

- パース済み: `ScriptList`, `FeatureList`, `LookupList`
- [+] 実装済み: 単一置換ベースの縦書き置換 `lookup_vertical()`
- [*] 部分実装: `ccmp` sequence 適用は text API shaping で利用、`lookup_ccmp()` の個別 API は未整理
- [+] 実装済み: `lookup_locale()`, `lookup_liga()`
- [*] text API への反映: `text2command()`, `text2commands()`, `measure()` で variation selector と基本的な `locl` / `liga` / `dlig` / `ccmp` を利用
    - [x] `TextDirection::TopToBottom` で縦メトリクスと縦書き置換を利用
    - [x] `TextDirection::RightToLeft` で RTL の inline 進行方向を利用
    - [*] `FontVariant` から日本語 variant form を要求
        - [x] `jp78` を実フォント確認
        - [*] `jp90` / `trad` / `nlck` は API 実装済みだが実フォント確認は未完
    - [x] Arabic joining (`isol` / `init` / `medi` / `fina`)
    - [x] Arabic required ligature (`rlig`)
    - [x] Arabic contextual substitutions (`rclt` / `calt` / `clig`)
    - [x] locale/script 優先 + required feature を含む lookup 選択
    - [*] context/chaining ベースの RTL shaping
        - [x] Context Format 1 / 2 / 3 の feature-sequence 適用
        - [x] Chaining Context Format 1 / 2 / 3 の feature-sequence 適用
        - [*] script 固有の chaining / language-specific lookup 拡張
            - [x] full locale subtag から language system を選択
            - [ ] 実フォントの script 固有 chaining を追加
- [ ] 未実装: `lookup_width()`, `lookup_number()`
- [ ] aalt
- [ ] dlig
- [ ] expt
- [ ] fwid
- [ ] hwid
- [*] jp78
- [*] jp90
- [ ] llga
- [*] nlck
- [ ] pwid
- [*] trad
- [ ] vert
- [ ] vrt2
- [ ] zero


### Lookup パース
- [x] Type 1 Single Substitution: パース済み、展開可能
- [x] Type 2 Multiple Substitution: パース済み、展開可能
- [ ] Type 3 Alternate Substitution: パース済み、展開可能
- [x] Type 4 Ligature Substitution: パース済み、展開可能
- [ ] Type 5 Context Substitution:
    - [x] Format 1
    - [x] Format 2
    - [x] Format 3
- [ ] Type 6 Chaining Context Substitution:
    - [x] Format 1
    - [x] Format 2 はパース済みで、feature-sequence 適用器から部分適用可能
    - [x] Format 3
- [*] Type 7 Extension Substitution: パース済み、single / multiple / ligature は適用可能
- [ ] Type 8 Reverse Chaining Contextual Single Substitution: パース済み、適用は未実装

### GDEF
- [ ]パース済み: glyph class definition, attach list, ligature caret list, mark attach class definition, mark glyph sets definition
- [] shaping 処理に統合


# Font table
- [x] font table
  - **MUST**
  - [x] cmap
  - [x] head
  - [x] hhea
  - [x] hmtx
  - [x] name
  - [x] OS/2
  - [x] post
  - **OPTIONS**
    - [x] maxp
    - [x] 'vhea'	Vertical Metrics header **MAST**
    - [x] 'vmtx'	Vertical Metrics **MAST**
    - [ ] cvt
    - [ ] fpgm
    - [x] glyf **MUST**
    - [ ] prep
    - [ ] gasp
    - [x] CFF **MUST**
    - [ ] CFF2 **SHOUD**
    - [ ] VORG
  - Advanced Typographic Tables
    - [+] GDEF
    - [+] GSUB -> see lookup, coverage, classdef, language
    - [+] GPOS -> see lookup, coverage, classdef, language
    - [ ] BASE
    - [ ] JSTF
    - [ ] MATH
  - Bitmap
    - [ ] EBDT
    - [ ] EBLC
    - [ ] EBSC
  - COLOR
    - [x] COLR **MUST**
    - [x] CPAL **MUST**
    - [ ] CBDT
    - [ ] CBLC
    - [x] `sbix` **MUST**
    - [ ] SVG **SHOULD**
        - [x] getter
        - [x] svg divider
  - OTHERS
    - [ ] DSIG	Digital signature
    - [ ] 'hdmx'	Horizontal device metrics
    - [ ] 'kern'	Kerning
    - [ ] LTSH	Linear threshold data
    - [ ] MERG	Merge
    - [ ] 'meta'	Metadata
    - [ ] STAT	Style attributes
    - [ ] PCLT	PCL 5 data
    - [ ] VDMX	Vertical device metrics

# todo.mdの更新

# 追加バックログ

## メンテナンス / 整理
- [*] dead codeの削除または隔離（`src/woff/woff2.rs` など）
  - [x] 未参照だった `src/woff/woff2.rs` を削除
  - [x] default build で不要な低レイヤ dead code を feature 境界の内側へ隔離
- [*] `FontFamily` のフォールバック / GPOS / GDEF / GSUB の適用順序を整理
  - [x] GSUB sequence stage と ligature stage を分離して順序を明示
  - [x] GPOS pair positioning の前後 glyph 探索に GDEF mark skip を導入
  - [x] fallback face 選択に text direction / locale / font variant を反映
  - [*] fallback face をまたぐ script / language / mark attachment の扱いを整理
    - [x] combining mark を fallback text unit として分断しない
    - [x] RTL contextual script では同一 face を優先して segment continuity を維持
    - [x] Arabic / Syriac の real font fixture で face 切替境界の回帰テストを追加
    - [x] Arabic / Syriac / Hebrew の複数フォント候補を走査する境界チェックを追加
    - [ ] 実フォントで script ごとの face 切替境界をさらに詰める

## examples / ドキュメント
- [*] examplesのテスト
  - [x] 修正だけで済むか確認
  - [x] `--features raw` が必要か不要かを example ごとに判定
  - [x] 判定結果を `README.md` / `README.ja.md` に反映
  - [x] 既存 common helper ベースで、主要なパス / ファイル名指定が引数化されていることを確認
  - [x] public API の corpus smoke test を追加して metadata / shape / render_svg を実フォント群で確認
- [x] `fontmetadata.rs` を追加して metadata 表示用 example を用意
- [x] READMEの整理
  - [x] APIとsample中心に書き直す
  - [x] 今の技術資料寄りの細かい仕様は `doc/` 配下へ移動
  - [x] `cargo doc` 向けに公開APIの rustdoc を追加

## CI / CD
- [x] GitHub Workflows を作成
  - [x] examplesのbuildを含める
  - [x] 対象: Windows x86 / arm
  - [x] 対象: Linux x64 / arm
  - [x] 対象: Mac x64 / arm
  - [x] tag push時に起動

## shaping / script
- [ ] アラビア語フォントの対応を進める
  - [x] `FontEngine::with_shaping_policy()` と `ShapingPolicy` で LTR / RTL / vertical を公開APIに明示
  - [*] Arabic / Syriac の mark attachment を GDEF attach class まで使って詰める
    - [x] `mark_attachment_class()` / `attach_point_indices()` を GDEF に追加
    - [x] attachable mark では前の base に重ねる fallback 位置決めを導入
    - [x] boundary test を複数フォント候補へ拡張
    - [ ] GPOS mark-to-base / mark-to-mark 相当の精度までは未対応
- [ ] スクリプト文字対応

## format
- [ ] CFF2対応を進める
  - [x] CFF INDEX の `count + 1` overflow を修正して大規模 CFF collection を通せるようにした
  - [x] `CFF2` table を outline format として認識し、未対応時は panic ではなく unsupported へ倒すようにした
  - [x] CFF2 本体の実装前に、CFF / TTC / WOFF / WOFF2 をまたぐ corpus smoke test を追加
  - [x] `fvar` / `avar` / `HVAR` / `VVAR` / `MVAR` を読み込み、variable font の public API metadata と metrics variation を通した
  - [x] `FontEngine::with_variation()` と `FontFace::variation_axes()` を追加
  - [x] 実フォント fixture で variable axis metadata と `wdth` による measure 変化を回帰テスト化
  - [x] `Invalid delta format` だった variable font fixture は skip 前提を外した
  - [x] `gvar` simple glyph の outline delta を実装し、`FontEngine::shape()` の outline に反映
  - [x] real variable-font fixture で outline signature の変化を回帰テスト化
  - [x] CFF2 実装前の共有化調査を `doc/cff2-investigation*.md` に追加
  - [x] composite glyph の `gvar` delta は再帰 flatten + component variation 適用で対応
  - [ ] phantom point 由来の outline / metrics 補正は未対応
  - [x] CFF2 charstring / variation store / blend operator 本体を `cff.rs` 共有経路に実装
  - [*] local corpus に true CFF2 実フォントがまだ無く、現状の CFF2 coverage は synthetic test 中心。実フォント fixture 入手後に `shape()` / `render_svg()` smoke を増やしたい
  - [x] CFF2 の Private DICT `vsindex` / `blend` は parser 側に実装済み
