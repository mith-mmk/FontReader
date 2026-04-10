# APIレシピ

公開APIの実行例を、用途ごとにまとめたドキュメントです。

## Metadata を読む

```rust
use fontcore::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
println!("{}", face.family());
println!("{}", face.full_name());
println!("{}", face.weight().0);
println!("{}", face.is_italic());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## 基本の shaping

```rust
use fontcore::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
let run = face.engine().with_font_size(32.0).shape("Hello")?;
assert!(!run.glyphs.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## 文字幅を測る

```rust
use fontcore::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
let width = face.engine().with_font_size(32.0).measure("Hello")?;
assert!(width > 0.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## 縦書き SVG 出力

```rust
use fontcore::FontFile;

let face = FontFile::from_file("fonts/YourFont.otf")?.current_face()?;
let svg = face
    .engine()
    .with_font_size(32.0)
    .with_vertical_flow()
    .render_svg_vertical("縦書き")?;
assert!(svg.contains("<svg"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## RTL shaping

```rust
use fontcore::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
let run = face
    .engine()
    .with_font_size(32.0)
    .with_right_to_left()
    .shape("مرحبا")?;
assert!(!run.glyphs.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## GSUB variant 切り替え

```rust
use fontcore::{FontFile, FontVariant};

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

## Variable font axis 指定

```rust
use fontcore::FontFile;

let face = FontFile::from_file("fonts/VariableFont.ttf")?.current_face()?;
for axis in face.variation_axes() {
    println!(
        "{} {}..{} (default {})",
        axis.tag, axis.min_value, axis.max_value, axis.default_value
    );
}

let width = face
    .engine()
    .with_font_size(32.0)
    .with_variation("wdth", 75.0)
    .measure("Hello")?;
assert!(width > 0.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## TTC / collection の face 選択

```rust
use fontcore::FontFile;

let file = FontFile::from_file("fonts/YourCollection.ttc")?;
let face = file.face(1)?;
println!("{}", face.full_name());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## FontFamily fallback

```rust
use fontcore::{FontFamily, FontFile, FontWeight};

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

## 分割 WOFF2 読み込み

```rust
use fontcore::ChunkedFontBuffer;

let mut buffer = ChunkedFontBuffer::new(total_size)?;
buffer.append(0, first_chunk)?;
buffer.append(second_offset, second_chunk)?;

if buffer.is_complete() {
    let face = buffer.into_font_face()?;
    assert!(face.measure("Hello")? > 0.0);
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## メモ

- shaping direction、locale、variant、variable-font axis は `FontEngine` を主な入口にしています。
- `FontOptions` は低レイヤ寄りの制御や `FontFamily` 連携用として残しています。
- 実装状況と制限事項は [feature-status.ja.md](feature-status.ja.md) にまとめています。
- CFF2 調査メモは [cff2-investigation.ja.md](cff2-investigation.ja.md) にまとめています。
- ドキュメントの入口は [README.ja.md](README.ja.md) です。
