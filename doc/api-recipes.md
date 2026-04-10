# API Recipes

This document keeps runnable public-API examples grouped by task.

## Read Metadata

```rust
use fontloader::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
println!("{}", face.family());
println!("{}", face.full_name());
println!("{}", face.weight().0);
println!("{}", face.is_italic());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Basic Shaping

```rust
use fontloader::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
let run = face.engine().with_font_size(32.0).shape("Hello")?;
assert!(!run.glyphs.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Measure Text

```rust
use fontloader::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
let width = face.engine().with_font_size(32.0).measure("Hello")?;
assert!(width > 0.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Vertical SVG Output

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

## RTL Shaping

```rust
use fontloader::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
let run = face
    .engine()
    .with_font_size(32.0)
    .with_right_to_left()
    .shape("مرحبا")?;
assert!(!run.glyphs.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## GSUB Variant Switching

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

## Variable Font Axes

```rust
use fontloader::FontFile;

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

## TTC Or Collection Access

```rust
use fontloader::FontFile;

let file = FontFile::from_file("fonts/YourCollection.ttc")?;
let face = file.face(1)?;
println!("{}", face.full_name());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## FontFamily Fallback

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

## Chunked WOFF2 Loading

```rust
use fontloader::ChunkedFontBuffer;

let mut buffer = ChunkedFontBuffer::new(total_size)?;
buffer.append(0, first_chunk)?;
buffer.append(second_offset, second_chunk)?;

if buffer.is_complete() {
    let face = buffer.into_font_face()?;
    assert!(face.measure("Hello")? > 0.0);
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Notes

- `FontEngine` is the main place to choose shaping direction, locale, variants, and variable-font axes.
- `FontOptions` still exists for lower-level control and `FontFamily` integration.
- Current implementation notes and limitations live in [feature-status.md](feature-status.md).
- CFF2 investigation notes live in [cff2-investigation.md](cff2-investigation.md).
- For a document index, start at [README.md](README.md).
