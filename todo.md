# todo
fonts/* がフォント
- [+] 実装済み
- [x] 動作確認済み
- [*] 実装済みだが動作に不具合あり
- [-] 実装遅延
- [ ] 未処理、未確認タスク

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
    - [ ] otf　```Error: Custom { kind: Other, error: "glyf is none" }``` by notosans
    - [ ] woffで以下のエラーが出るissue
```
 cargo run --example fontloader -- -f .\fonts\MS-Gothic.ttf.woff
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s
     Running `C:\Users\misir\rust-targets\fontloader\debug\examples\fontloader.exe -f .\fonts\MS-Gothic.ttf.woff`
tag: BASE 42415345 offset: 000001D0 comp_length: 166 orig_length: 376 orig_checksum: CA352087
tag: EBDT 45424454 offset: 00000278 comp_length: 735407 orig_length: 1358015 orig_checksum: 78839180
tag: EBLC 45424C43 offset: 000B3B28 comp_length: 23552 orig_length: 196072 orig_checksum: 33A67753
tag: GSUB 47535542 offset: 000B9728 comp_length: 1282 orig_length: 1600 orig_checksum: 4F2C24BA
tag: OS/2 4F532F32 offset: 000B9C2C comp_length: 76 orig_length: 86 orig_checksum: 539E6FE0
tag: cmap 636D6170 offset: 000B9C78 comp_length: 8522 orig_length: 11872 orig_checksum: 62F54F37
tag: cvt  63767420 offset: 000BBDC4 comp_length: 175 orig_length: 800 orig_checksum: 0B9A0957
tag: fpgm 6670676D offset: 000BBE74 comp_length: 734 orig_length: 1313 orig_checksum: 105D8206
tag: gasp 67617370 offset: 000BC154 comp_length: 16 orig_length: 16 orig_checksum: 001F0009
tag: glyf 676C7966 offset: 000BC164 comp_length: 1497668 orig_length: 2431326 orig_checksum: 9608EC84
tag: head 68656164 offset: 00229BA8 comp_length: 54 orig_length: 54 orig_checksum: B616127D
tag: hhea 68686561 offset: 00229BE0 comp_length: 33 orig_length: 36 orig_checksum: 01C03405
tag: hmtx 686D7478 offset: 00229C04 comp_length: 10041 orig_length: 52416 orig_checksum: 414A2E8A
tag: loca 6C6F6361 offset: 0022C340 comp_length: 24527 orig_length: 52420 orig_checksum: 7206B130
tag: maxp 6D617870 offset: 00232310 comp_length: 32 orig_length: 32 orig_checksum: 37F71295
tag: mort 6D6F7274 offset: 00232330 comp_length: 1274 orig_length: 1608 orig_checksum: 401263FA
tag: name 6E616D65 offset: 0023282C comp_length: 620 orig_length: 1589 orig_checksum: 524CD230
tag: post 706F7374 offset: 00232A98 comp_length: 19 orig_length: 32 orig_checksum: FFF20013 
tag: prep 70726570 offset: 00232AAC comp_length: 1827 orig_length: 7666 orig_checksum: 7854F728
tag: vhea 76686561 offset: 002331D0 comp_length: 26 orig_length: 36 orig_checksum: 01DE3454
tag: vmtx 766D7478 offset: 002331EC comp_length: 9487 orig_length: 52416 orig_checksum: E5517310
metadata:

thread 'main' (62676) panicked at examples\fontloader.rs:10:56:
called `Result::unwrap()` on an `Err` value: Custom { kind: Other, error: "ountbound call ptr 86 + 2 but buffer length 86" }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
error: process didn't exit successfully: `C:\Users\misir\rust-targets\fontloader\debug\examples\fontloader.exe -f .\fonts\MS-Gothic.ttf.woff` (exit code: 101)
```

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
  - [+] font.text2svg(&self, &str, size: Option<f32>) -> String // textをsvgに変換して返す 
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
- load_font フォントロード form any
- [ ] load_font_from_file フォントロード from NET(exclude WASM)
- [ ] load_font_from__buffer フォントロード from buffer
- [+] load_font_from__net フォントロード from NET(exclude WASM)

# 合字対応
- [ ] llaga
- [ ] アラビア文字
- [ ] 日本語
- [ ] チベット語
- [ ] 古ハングル
- [ ] その他
# format
- [+] woff2対応
- [ ] 境界条件をチェックしpanic!を回避
- [ ] svg svgのサイズが巨大なので文字毎にsvgを切り出す
# Layout 対応状況

`layout` feature は一部のみ実装されています。

- [ ] layout featureの拡張

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
