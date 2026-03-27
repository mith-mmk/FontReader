# Rust向け Fontloader

OpenType、TrueType、TTC、WOFF、および一部の WOFF2 フォントデータを読み込むための Rust ライブラリです。

English: [README.md](README.md)

## GlyphRun API

`src/commands.rs` に対応した `fontloader::text2commands(text, FontOptions)` を追加し、
`GlyphRun` を直接生成できるようにしました。

- `FontOptions::new(&font)` でロード済みフォントをそのまま渡せます。
- `font_size` と `line_height` は px として解釈されます。
- `font_stretch`、`font_style`、`font_variant`、`font_weight` を `FontOptions` に保持できます。
- `layout` feature 有効時は `FontOptions::with_locale("ja-JP")` で GSUB `locl` を要求できます。
- `FontOptions::from_family(&family)` を使うと、キャッシュ済みの `FontFamily` から family/name/weight/style/stretch 条件で face を選べます。
- ただし現状の `FontFamily` は cache ベースの選択のみで、fallback chain や Last Resort 自動選択は未実装です。
- TrueType / CFF は `GlyphLayer::Path` として返します。
- `sbix` は `GlyphLayer::Raster` として返します。
- COLR/CPAL の色は `GlyphPaint::Solid(0xAARRGGBB)` に詰めて返すので、そのまま `paintcore::path::draw_glyphs` に渡せます。
- SVG glyph は現状 `ErrorKind::Unsupported` を返します。
- 既存の `font.text2command()` は輪郭コマンドだけを返す旧 API で、レイヤーごとの色は保持しません。カラーグリフを扱う場合は `fontloader::text2commands(..., FontOptions)` を使ってください。

## Renderer 連携仕様

`paintcore::path::draw_glyphs` のような描画系に渡す場合は、旧来の outline-only API ではなく
`GlyphRun` API を使ってください。

- `fontloader::text2commands(text, FontOptions)` または `LoadedFont::text2glyph_run()` を使います。
- `GlyphPaint::Solid(u32)` の色形式は `0xAARRGGBB` です。
- `GlyphPaint::CurrentColor` は「レンダラに渡した既定色を使う」という意味です。
- COLR/CPAL glyph はレイヤーごとの色を `GlyphPaint::Solid(...)` に保持します。
- `sbix` glyph は `GlyphLayer::Raster` として返します。
- `font.text2command()` / `font.text2commands()` が返す `Vec<GlyphCommands>` は旧来の輪郭互換 API です。
  こちらはレイヤー色、raster glyph、color font 情報を保持しません。

要点だけ言うと:

- 色付き描画に使うのは `GlyphRun`
- 輪郭だけ必要な互換用途は `GlyphCommands`

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

## FontFamily キャッシュ

`FontFamily` は、ロード済みフォントと `ChunkedFontBuffer` の上に置く取得・キャッシュ層です。

- 完全にロード済みの face は `add_loaded_font()` または `add_face(...)` で登録します。
- 分割取得中の face は `begin_chunked_face(face_id, descriptor, total_size)` で登録します。
- chunk は `append_chunk(face_id, offset, bytes)` で順不同に投入できます。
- 追加取得が必要な範囲は `missing_ranges(face_id)` で確認できます。
- すべてそろったら `finalize_chunked_face(face_id)` で cache に昇格します。
- shaping 時は `FontOptions::from_family(&family)` に
  `with_font_family(...)`, `with_font_name(...)`, `with_font_weight(...)`,
  `with_font_style(...)`, `with_font_stretch(...)` を組み合わせて face を解決します。

これは「並列取得して再構成する」ための層で、WOFF2 を真の lazy decode するものではありません。

```rust
use fontloader::{
    text2commands, FontFaceDescriptor, FontFamily, FontOptions, FontStyle, FontWeight,
};

let mut family = FontFamily::new("Fira Sans");
family.begin_chunked_face(
    "fira-black",
    FontFaceDescriptor::new("Fira Sans")
        .with_font_name("Fira Sans Black")
        .with_font_weight(FontWeight::BLACK)
        .with_font_style(FontStyle::Normal),
    total_size,
)?;

family.append_chunk("fira-black", 0, first_chunk)?;
family.append_chunk("fira-black", next_offset, second_chunk)?;

if family.missing_ranges("fira-black")?.is_empty() {
    family.finalize_chunked_face("fira-black")?;
}

let run = text2commands(
    "Hello",
    FontOptions::from_family(&family)
        .with_font_family("Fira Sans")
        .with_font_weight(FontWeight::BLACK),
)?;
assert!(!run.glyphs.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

既存の `fontload*` API に加えて、`load_font`、`load_font_from_file`、
`load_font_from_buffer` エイリアスも使えます。

## 分割フォント読み込み

並列取得や range request でフォントを集める場合は、`ChunkedFontBuffer` で完全な
buffer に再構成してから decode できます。

- WOFF2 が複数の byte range に分かれて届くケースを想定しています。
- 現在の WOFF2 decode は、完全な byte stream がそろってから実行する前提です。
- `append(offset, bytes)` は順不同の chunk を受け付けます。
- `missing_ranges()` で未取得の範囲を確認できます。
- `into_loaded_font()` / `load_font()` で既存 loader に渡せます。

```rust
use fontloader::ChunkedFontBuffer;

let mut buffer = ChunkedFontBuffer::new(total_size)?;
buffer.append(1024, chunk_b)?;
buffer.append(0, chunk_a)?;

if buffer.is_complete() {
    let font = buffer.into_loaded_font()?;
    let width = font.measure("Hello")?;
    assert!(width > 0.0);
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

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
- 実装済み: `lookup_locale()`, `lookup_liga()`
- text API: `text2command()`, `text2commands()`, `measure()` で variation selector と基本的な `locl` / `liga` / `dlig` shaping を利用
- 未実装: `lookup_width()`, `lookup_number()`

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
