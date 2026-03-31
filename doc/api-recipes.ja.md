# APIレシピ

公開APIの使い方をまとめたメモです。

## 基本の shaping

```rust
use fontloader::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
let run = face.engine().with_font_size(32.0).shape("Hello")?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## 縦書き SVG 出力

```rust
use fontloader::FontFile;

let face = FontFile::from_file("fonts/YourFont.otf")?.current_face()?;
let svg = face
    .engine()
    .with_font_size(32.0)
    .with_vertical_flow()
    .render_svg_vertical("縦書き")?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## GSUB variant 切り替え

```rust
use fontloader::{FontFile, FontVariant};

let face = FontFile::from_file("fonts/YourFont.otf")?.current_face()?;
let run = face
    .engine()
    .with_font_size(32.0)
    .with_locale("ja-JP")
    .with_font_variant(FontVariant::Jis78)
    .shape("辻")?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## TTC / collection の face 選択

```rust
use fontloader::FontFile;

let file = FontFile::from_file("fonts/YourCollection.ttc")?;
let face = file.face(1)?;
println!("{}", face.full_name());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## メモ

- shaping direction や variant の指定は `FontEngine` を主な入口にする想定です。
- `FontOptions` は `FontFamily` と組み合わせる低レイヤ寄りの制御として残しています。
- 実装メモや現時点の制限事項は `feature-status.ja.md` にまとめています。
