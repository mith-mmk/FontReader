# Rust向け Fontloader

Fontloader は、フォントを読み込み、文字列を shaping して `GlyphRun` や SVG に変換するための Rust ライブラリです。

English: [README.md](README.md)

## 公開API

- `FontFile`
  - フォントファイル / コレクションの入口
  - TTC などから face を選ぶ
- `FontFace`
  - 1 face を表す
  - `family()`, `full_name()`, `weight()`, `is_italic()` などの metadata を持つ
- `FontEngine`
  - shaping と描画担当
  - `shape(text)`, `measure(text)`, `render_svg(text)` を持つ
- `FontFamily`
  - 複数 face のキャッシュと face 選択、glyph fallback を担当

低レイヤの parser API は `features = ["raw"]` で利用できます。

## 対応フォーマット

- TrueType
- OpenType / CFF
- TTC
- WOFF
- WOFF2

default feature には `layout` と `cff` が含まれます。

## 最小サンプル

```rust
use fontloader::{FontFile, ShapingPolicy};

let file = FontFile::from_file("fonts/YourFont.ttf")?;
let face = file.current_face()?;
let engine = face
    .engine()
    .with_shaping_policy(ShapingPolicy::LeftToRight)
    .with_font_size(32.0)
    .with_line_height(40.0)
    .with_svg_unit("px");

let run = engine.shape("Hello")?;
let width = engine.measure("Hello")?;
let svg = engine.render_svg("Hello")?;

println!("{}", face.family());
println!("{}", width);
println!("{}", svg);
println!("{}", run.glyphs.len());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## よく使うAPIパターン

縦書き shaping / SVG 出力:

```rust
use fontloader::FontFile;

let face = FontFile::from_file("fonts/YourFont.otf")?.current_face()?;
let svg = face
    .engine()
    .with_font_size(32.0)
    .with_vertical_flow()
    .render_svg_vertical("縦書き")?;

assert!(svg.contains("<svg"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

GSUB variant 切り替え:

```rust
use fontloader::{FontFile, FontVariant};

let face = FontFile::from_file("fonts/YourFont.otf")?.current_face()?;
let run = face
    .engine()
    .with_font_size(32.0)
    .with_locale("ja-JP")
    .with_font_variant(FontVariant::Jis78)
    .shape("辻")?;

assert!(!run.glyphs.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Variable font axis 指定:

```rust
use fontloader::FontFile;

let face = FontFile::from_file("fonts/VariableFont.ttf")?.current_face()?;
let width = face
    .engine()
    .with_font_size(32.0)
    .with_variation("wdth", 75.0)
    .measure("Hello")?;

println!("{width}");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## フォントの読み込み

```rust
use fontloader::{load_font_from_buffer, FontFile};

let bytes = std::fs::read("fonts/YourFont.ttf")?;

let face = load_font_from_buffer(&bytes)?;
let file = FontFile::from_buffer(&bytes)?;
assert!(file.face_count() >= 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

TTC / collection の場合:

```rust
use fontloader::FontFile;

let file = FontFile::from_file("fonts/YourCollection.ttc")?;
let face0 = file.face(0)?;
let face1 = file.face(1)?;

println!("{}", face0.full_name());
println!("{}", face1.full_name());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## FontFamily

`FontFamily` は高レベルの cache / fallback 層です。

```rust
use fontloader::{FontFamily, FontFile, FontWeight};

let regular = FontFile::from_file("fonts/FiraSans-Regular.ttf")?.current_face()?;
let bold = FontFile::from_file("fonts/FiraSans-Bold.ttf")?.current_face()?;

let mut family = FontFamily::new("Fira Sans");
family.add_font_face(regular);
family.add_font_face(bold);

let run = family.text2glyph_run(
    "Hello",
    family.options().with_font_weight(FontWeight::BOLD),
)?;

assert!(!run.glyphs.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## 分割 WOFF2 / range request

```rust
use fontloader::ChunkedFontBuffer;

let mut buffer = ChunkedFontBuffer::new(total_size)?;
buffer.append(0, first_chunk)?;
buffer.append(second_offset, second_chunk)?;

if buffer.is_complete() {
    let face = buffer.into_font_face()?;
    let width = face.measure("Hello")?;
    assert!(width > 0.0);
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## examples

example は共通の CLI 引数を持ちます。

- `-f`, `--font`: フォントファイル
- `-d`, `--dir`: フォントディレクトリ
- `-i`, `--index`: コレクション中の face index
- `-o`, `--output`: 出力ファイル
- `-s`, `--string`: 文字列を直接指定
- `-t`, `--text-file`: テキストファイルを指定
- `--vertical`: 縦書きで出力
- `--variant`: `jp78`, `jp90`, `trad`, `nlck` などの GSUB variant 指定

`raw` なしで使える高レベル example:

- `api_overview`
- `fontmetadata`
- `fontloader`

`--features raw` が必要な inspection / 旧API example:

- `fontcmaps`
- `fontcolor`
- `fontgkana`
- `fontgsub`
- `fontheader`
- `fontload`
- `fontname`
- `fontsbix`
- `fontsvg`
- `fonttest`
- `fonttype`
- `tategaki`

高レベル example:

```bash
cargo run --example api_overview -- -f path/to/font.ttf -s "Hello"
```

`raw` example:

```bash
cargo run --example fonttype --features raw -- -d path/to/fonts
```

## WebAssembly

`wasm32-unknown-unknown` でもコンパイルできます。

- `load_font_from_buffer()` または `load_font(FontSource::Buffer(...))` を使ってください
- `load_font_from_file()` と `load_font_from_net()` は `wasm32` では `ErrorKind::Unsupported`

## raw API

旧来の低レイヤAPIが必要な場合は `raw` feature を有効にします。

```toml
[dependencies]
fontloader = { version = "0.0.10", features = ["raw"] }
```

これで以下が有効になります。

- `fontloader::Font`
- `fontloader::fontheader`
- `fontloader::opentype`
- `fontload_*` などの deprecated 互換 API

## 詳細資料

- APIレシピ: [doc/api-recipes.ja.md](doc/api-recipes.ja.md)
- 実装メモ / 現在の format 対応状況: [doc/feature-status.ja.md](doc/feature-status.ja.md)
- CFF2 調査メモ: [doc/cff2-investigation.ja.md](doc/cff2-investigation.ja.md)
