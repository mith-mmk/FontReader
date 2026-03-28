# todo
_test* が作業用フォルダ
_test_fonts/* がテスト用フォント
- [+] 実装済み
- [x] 動作確認済み
- [*] 実装済みだが動作に不具合あり
- [-] 実装遅延
- [ ] 未処理、未確認タスク

# 最優先
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
- [ ] アラビア文字
- [ ] 日本語
- [ ] チベット語
- [ ] 古ハングル
- [ ] その他
# format
- [+] woff2対応
- [+] CID-keyed CFF / FDSelect
- [ ] 境界条件をチェックしpanic!を回避
- [ ] svg svgのサイズが巨大なので文字毎にsvgを切り出す
# Layout 対応状況

`layout` feature は一部のみ実装されています。

- [*] layout featureの拡張
    - [x] text2command / text2commands / measure で variation selector と基本合字(liga / dlig) を利用
    - [x] `FontOptions::with_locale()` から `locl` shaping を利用
    - [ ] ccmp / context chaining などの shaping 拡張

# opentype
- [x] True Type
- [x] cff
- [ ] cff2
- [x] color true type
- [+] sbix
- [ ] svg  # svgパーサーがいる


# GPOS
- [ ] 実装
- [ ] palt
- [ ] vpal
- [ ] kern
- [ ] vkrn
- [ ] halt
- [ ] vhal

# GSUB

- パース済み: `ScriptList`, `FeatureList`, `LookupList`
- [+] 実装済み: 単一置換ベースの縦書き置換 `lookup_vertical()`
- [ ] 部分実装: `lookup_ccmp()` は存在するが、結果展開は未実装
- [+] 実装済み: `lookup_locale()`, `lookup_liga()`
- [*] text API への反映: `text2command()`, `text2commands()`, `measure()` で variation selector と基本的な `locl` / `liga` / `dlig` を利用
- [ ] 未実装: `lookup_width()`, `lookup_number()`
- [ ] aalt
- [ ] dlig
- [ ] expt
- [ ] fwid
- [ ] hwid
- [ ] jp78
- [ ] jp90
- [ ] llga
- [ ] nlck
- [ ] pwid
- [ ] trad
- [ ] vert
- [ ] vrt2
- [ ] zero


### Lookup パース
- [ ] Type 1 Single Substitution: パース済み、展開可能
- [ ] Type 2 Multiple Substitution: パース済み、展開可能
- [ ] Type 3 Alternate Substitution: パース済み、展開可能
- [ ] Type 4 Ligature Substitution: パース済み、展開可能
- [ ] Type 5 Context Substitution:
    - [ ] Format 1 は展開可能
    - [ ] Format 2
    - [ ] Format 3
- [ ] Type 6 Chaining Context Substitution:
    - [ ] Format 1 は展開可能
    - [ ] Format 2 は一部のみ適用
    - [ ] Format 3 はパースのみで、適用は未実装
- [ ] Type 7 Extension Substitution: パース済み、適用は未実装
- [ ] Type 8 Reverse Chaining Contextual Single Substitution: パース済み、適用は未実装

### GDEF
- [ ]パース済み: glyph class definition, attach list, ligature caret list, mark attach class definition, mark glyph sets definition
- [] shaping 処理に統合
  
# todo.mdの更新
