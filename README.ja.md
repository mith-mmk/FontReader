# Rust向け Fontloader

OpenType、TrueType、TTC、WOFF、および一部の WOFF2 フォントデータを読み込むための Rust ライブラリです。

English: [README.md](README.md)

## GlyphRun API

`src/commands.rs` に対応した `fontloader::text2commands(text, FontOptions)` を追加し、
`GlyphRun` を直接生成できるようにしました。

- `FontOptions::new(&font)` でロード済みフォントをそのまま渡せます。
- `font_size` と `line_height` は px として解釈されます。
- `font_stretch`、`font_style`、`font_variant`、`font_weight` を `FontOptions` に保持できます。
- `font-family` / `font-name` でのフォント探索は未実装なので、当面はロード済みフォントを渡してください。
- TrueType / CFF は `GlyphLayer::Path` として返します。
- `sbix` は `GlyphLayer::Raster` として返します。
- COLR/CPAL の色は `GlyphPaint::Solid(0xAARRGGBB)` に詰めて返すので、そのまま `paintcore::path::draw_glyphs` に渡せます。
- SVG glyph は現状 `ErrorKind::Unsupported` を返します。
- 既存の `font.text2command()` は輪郭コマンドだけを返す旧 API で、レイヤーごとの色は保持しません。カラーグリフを扱う場合は `fontloader::text2commands(..., FontOptions)` を使ってください。

```rust
use fontloader::{load_font_from_buffer, text2commands, FontOptions, GlyphLayer};

let bytes = std::fs::read("fonts/ZenMaruGothic-Regular.ttf")?;
let font = load_font_from_buffer(&bytes)?;
let run = text2commands(
    "Hello\nWorld",
    FontOptions::new(&font)
        .with_font_size(32.0)
        .with_line_height(40.0),
)?;

for glyph in &run.glyphs {
    for layer in &glyph.glyph.layers {
        match layer {
            GlyphLayer::Path(path) => {
                println!("path commands: {}", path.commands.len());
            }
            GlyphLayer::Raster(_) => {
                println!("bitmap glyph");
            }
        }
    }
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

既存の `fontload*` API に加えて、`load_font`、`load_font_from_file`、
`load_font_from_buffer` エイリアスも使えます。

## WebAssembly

ライブラリは `wasm32-unknown-unknown` 向けにコンパイル可能になりました。

- WebAssembly では `load_font_from_buffer()` または `load_font(FontSource::Buffer(...))` を使ってください。
- `load_font_from_file()` と `load_font_from_net()` は `wasm32` では `ErrorKind::Unsupported` を返します。

## Layout 対応状況

`layout` feature は一部のみ実装されています。

### GSUB

- パース済み: `ScriptList`, `FeatureList`, `LookupList`
- 実装済み: 単一置換ベースの縦書き置換 `lookup_vertical()`
- 部分実装: `lookup_ccmp()` は存在するが、結果展開は未実装
- 未実装: `lookup_locale()`, `lookup_liga()`, `lookup_width()`, `lookup_number()`

### Lookup パース

- Type 1 Single Substitution: パース済み、展開可能
- Type 2 Multiple Substitution: パース済み、展開可能
- Type 3 Alternate Substitution: パース済み、展開可能
- Type 4 Ligature Substitution: パース済み、展開可能
- Type 5 Context Substitution:
  Format 1 は展開可能
  Format 2 と Format 3 はパースのみで、適用は未完成
- Type 6 Chaining Context Substitution:
  Format 1 は展開可能
  Format 2 は一部のみ適用
  Format 3 はパースのみで、適用は未実装
- Type 7 Extension Substitution: パース済み、適用は未実装
- Type 8 Reverse Chaining Contextual Single Substitution: パース済み、適用は未実装

### GDEF

- パース済み: glyph class definition, attach list, ligature caret list, mark attach class definition, mark glyph sets definition
- 現状: 読み込みとデバッグ出力は可能だが、上位の shaping 処理にはまだ統合されていません

## examples の実行方法

example は `examples/` 以下にあります。

通常の実行:

```bash
cargo run --example fontloader -- path/to/font.ttf
```

layout パースが必要な example:

```bash
cargo run --features layout --example fontgsub -- path/to/font.ttf
```

CFF 対応が必要な example:

```bash
cargo run --features cff --example fontsvg -- path/to/font.otf
```

feature をまとめて有効にする場合:

```bash
cargo run --features full --example fontgsub -- path/to/font.otf
```

フォントパスを省略すると、example によっては OS の既定フォントを使います。
