# Rust向け fontcore

fontcore は、フォントを読み込み、face を選び、文字列を shaping し、SVG を出力するための Rust ライブラリです。

English: [README.md](README.md)

## 最初に使うAPI

通常は次の 3 つから始めれば十分です。

- `FontFile`
  - フォントファイル、TTC、メモリ上の bytes を開く入口
  - face を選ぶ
- `FontFace`
  - 1 face を表す
  - `family()`, `full_name()`, `weight()`, `is_italic()` などの metadata を持つ
- `FontEngine`
  - shaping、measure、SVG 出力を担当
  - 方向、locale、variant、variable-font axis の指定もここが主な入口

旧来の低レイヤ parser API は `features = ["raw"]` で引き続き利用できます。

## 対応フォーマット

- TrueType
- OpenType / CFF
- TTC
- WOFF
- WOFF2

default feature には `layout` と `cff` が含まれます。

## feature

- `layout`
  - GSUB / GPOS を使った shaping
  - 縦書き、RTL、locale、variant 指定
- `cff`
  - OpenType / CFF outline
- `raw`
  - 旧来の低レイヤ parser API
- `svg-fonts`
  - OpenType `SVG ` テーブルを glyph layer に変換する暫定サポート
  - 現状は `EmojiOneColor.otf` と `NotoColorEmoji-Regular.ttf` を主対象に回帰テスト済み
  - 単純 shape は path 化し、path 化できない payload は `GlyphLayer::Svg` として保持
  - path への完全展開や CSS / text 解釈は未対応

## 導入

```toml
[dependencies]
fontcore = "0.0.12"
```

低レイヤ parser API も必要な場合:

```toml
[dependencies]
fontcore = { version = "0.0.12", features = ["raw"] }
```

SVG emoji font の暫定サポートも使う場合:

```toml
[dependencies]
fontcore = { version = "0.0.12", features = ["svg-fonts"] }
```

## 最小サンプル

```rust
use fontcore::{FontFile, ShapingPolicy};

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
let engine = face
    .engine()
    .with_shaping_policy(ShapingPolicy::LeftToRight)
    .with_font_size(32.0)
    .with_svg_unit("px");

println!("{}", face.family());
println!("{}", engine.measure("Hello")?);
println!("{}", engine.render_svg("Hello")?);
println!("{}", engine.shape("Hello")?.glyphs.len());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## よく使う処理

- metadata 表示
  - `face.family()`
  - `face.full_name()`
  - `face.weight()`
  - `face.is_italic()`
- shaping
  - `engine.shape(text)`
- 幅の計測
  - `engine.measure(text)`
- SVG 出力
  - `engine.render_svg(text)`
- 縦書き
  - `engine.with_vertical_flow()`
- RTL shaping
  - `engine.with_right_to_left()`
- GSUB variant 指定
  - `engine.with_font_variant(...)`
- variable-font axis 指定
  - `face.variation_axes()`
  - `engine.with_variation("wght", 700.0)`

用途別の実行例は [doc/api-recipes.ja.md](doc/api-recipes.ja.md) にまとめています。

## SVG color font について

`sbix` は raster layer、`COLR/CPAL` は path layer、`SVG ` テーブルは `svg-fonts` 有効時のみ path layer 化を優先し、必要な場合だけ `Svg` layer を保持します。

現状の `svg-fonts` は、単純な `path` / `rect` / `circle` / `ellipse` / `line` / `polyline` / `polygon` を `PathGlyphLayer` に変換し、`defs` / `use`、`fill` / `fill-rule` / `stroke` / `stroke-width`、`clipPath` / `clip-path`、単純な `mask`、`translate` / `scale` / `rotate` / `skewX` / `skewY` / `matrix`、`linearGradient` / `radialGradient` / `stop`、`gradientUnits` / `gradientTransform` の最小保持まで入っています。path 化できない payload だけでなく、`pattern` / 複雑な `mask` / `filter` など未対応構文を含む payload も `GlyphLayer::Svg` fallback を併置します。

`paintcore` 側の renderer は 0.0.12 系の公開型に追随し、`clip_commands` と gradient paint も `fontcore -> paintcore` 変換で保持される前提です。

詳細仕様と `paintcore` への受け渡し境界は [doc/SVFONTSPEC.md](doc/SVFONTSPEC.md) にまとめています。

## examples

`raw` なしで使える高レベル example:

- `api_overview`
- `fontmetadata`
- `fontcore`

`--features raw` が必要な低レイヤ inspection example:

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

共通 CLI 引数:

- `-f`, `--font`: フォントファイル
- `-d`, `--dir`: フォントディレクトリ
- `-i`, `--index`: コレクション中の face index
- `-o`, `--output`: 出力ファイル
- `-s`, `--string`: 文字列を直接指定
- `-t`, `--text-file`: テキストファイル
- `--vertical`: 縦書き
- `--variant`: `jp78`, `jp90`, `trad`, `nlck` などの variant 指定

高レベル example:

```bash
cargo run --example api_overview -- -f path/to/font.ttf -s "Hello"
```

`raw` example:

```bash
cargo run --example fontheader --features raw -- -f path/to/font.otf
```

## `cargo doc`

公開API には rustdoc を付けています。

```bash
cargo doc --no-deps
```

そのまま開く場合:

```bash
cargo doc --no-deps --open
```

まず見るとよい型:

- `FontFile`
- `FontFace`
- `FontEngine`
- `FontFamily`
- `FontVariant`
- `ShapingPolicy`

## WebAssembly

`wasm32-unknown-unknown` でも利用できます。

- `load_font_from_buffer()` または `load_font(FontSource::Buffer(...))` を使ってください
- `load_font_from_file()` と `load_font_from_net()` は `wasm32` では `ErrorKind::Unsupported`

## ドキュメント一覧

- 追加ドキュメントの入口: [doc/README.ja.md](doc/README.ja.md)
- 公開APIレシピ: [doc/api-recipes.ja.md](doc/api-recipes.ja.md)
- 実装状況と制限事項: [doc/feature-status.ja.md](doc/feature-status.ja.md)
- CFF2 調査メモ: [doc/cff2-investigation.ja.md](doc/cff2-investigation.ja.md)
