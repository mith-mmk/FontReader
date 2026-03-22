# todo
# TESTの実装

# API
- [ ] FontFamiry Class
  - [ ] FontFamiryにフォールバックして探すシステム
    - [ ] 最下位にLast Resortが来る
- [ ] Font Class
  - LoadedFontのラッパー
    - defalut font size, fontのフォールバック情報などのデフォルト情報を持つ
  - [+] font.text2svg(&self, &str, size: Option<f32>) -> String // textをsvgに変換して返す 
  - font.text2command(&self, &str, size: Option<f32>) -> &FontCommand // textをコマンドにして返す
  - font.ritchtext2svg(&self, &FontText, size: Option<f32>) -> String // textをsvgに変換して返す 
  - font.ritchtext2command(&self, &FontText, size: Option<f32>) -> &FontCommand // textをコマンドにして返す

  - font.measure(FontText) -> f32 // 長さ(px)
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
  width: f32,
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
- load_font フォントロード form any
- load_font_from_file フォントロード from NET(exclude WASM)
- load_font_from__buffer フォントロード from buffer
- load_font_from__net フォントロード from NET(exclude WASM)

# format
- woff2対応

# Layout 対応状況

`layout` feature は一部のみ実装されています。

### GSUB

- パース済み: `ScriptList`, `FeatureList`, `LookupList`
- 実装済み: 単一置換ベースの縦書き置換 `lookup_vertical()`
- 部分実装: `lookup_ccmp()` は存在するが、結果展開は未実装
- 未実装: `lookup_locale()`, `lookup_liga()`, `lookup_width()`, `lookup_number()`

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