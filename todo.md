# todo
fonts/* がフォント
- [+] 実装済み
- [x] 動作確認済み
- [*] 実装済みだが動作に不具合あり
- [-] 実装遅延
- [ ] 未処理、未確認タスク
# issue
- fira_sansで i が読めない

# TESTの実装(最優先)
- [+] font load from file
- [+] font load from net
- [+] font load form buffer
- [+] text to svg
- [+] text to command
- [+] text measure
- [+] lookup (すべてのパターン)
- [+] cmap (すべてのパターン)
- [+] 異字体セレクタ
- [+] emoji
- [+] 合字
    - [ ] llga
    - [ ] 日本語 か+" など
    - [ ] チベット語
    - [ ] 古ハングル
    - [ ] その他
- [+] 縦書き
- loader
    - [ ] font
    - [ ] font collection
- [+] woff
- [+] woff2
- [+] otf (CID-keyed CFF / FDSelect)
- [+] woffで以下のエラーが出るissue OS2 Headerのoutbound


# Exampleの変更
- [+] ハードコーディングになっている部分を引数で渡せるようにする
- [+] txtを渡しているところは-s "string"で代替出来るようにする
  
# API
- [ ] FontFamiry Class
  - [ ] FontFamiryにフォールバックして探すシステム
    - [ ] 最下位にLast Resortが来る
    - [ ] FontFamiryでfont weight, font style itaric, bold, normalを切り替えられる様にする
      - [ ] font-size
      - [ ] font-stretch
      - [ ] font-style
      - [ ] font-variant
      - [ ] font-weight
      - [ ] line-height
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

- FontCommand
```
pub struct FontText {
  // todo!

}


pub struct FontCommand {
    pub glyph_count: usize,
    pub vertical: bool,
    pub line_spacing: f32,
    pub runs: Vec<FontRun>,
}

FontRun {
  font_id: usize,
  glyphs: Vec<GlyphInfo>
}

pub struct GlpyhInfo {
  glyph_id: u32,
  x: f32,
  y: f32,
  advance_width: f32,
  x_offset: f32,
  y_offset: f32,
  bbox: (f32,f32,f32,f32),
  width: f32,がでて動かなくなっています
  ascent: f32,
  descent: f32,
  // baseline_y 0.0
  color: u32,   // RGBA32 color for color font
  pub mod data: Vec<GlyphData>
}

pub enum GlyphType {
  PATH(Vec<GlyphData>), // open type, 
  IMAGE(Vec<u8>), //Image(apple emoji)
  SVG(String) , // google emoji
}


struct GlyphData {
  command : Vec<GlyphCommand>

}

pub enum GlyphCommand {
    Color(u8,u8,u8,u8),
    Line(f32, f32),
    MoveTo(f32, f32),
    QuadTo((f32, f32), (f32, f32)),
    CubicTo((f32, f32), (f32, f32), (f32, f32)),
    Fill,
    Close,
}

```


## APIの破壊的変更
- [+] load_font フォントロード from any
- [+] load_font_from_file フォントロード from file(exclude WASM)
- [+] load_font_from_buffer フォントロード from buffer
- [+] load_font_from_net フォントロード from NET(exclude WASM)

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

- [ ] layout featureの拡張

# opentype
- [+] True TYpe
- [+] cff
- [ ] cff2

# GSUB

- パース済み: `ScriptList`, `FeatureList`, `LookupList`
- [+] 実装済み: 単一置換ベースの縦書き置換 `lookup_vertical()`
- [ ] 部分実装: `lookup_ccmp()` は存在するが、結果展開は未実装
- [+] 実装済み: `lookup_locale()`, `lookup_liga()`
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

# GPOS
- [ ] 実装
- [ ] palt
- [ ] vpal
- [ ] kern
- [ ] vkrn
- [ ] halt
- [ ] vhal

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
