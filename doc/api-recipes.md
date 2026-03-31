# API Recipes

This document keeps the public API examples in one place.

## Basic shaping

```rust
use fontloader::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
let run = face.engine().with_font_size(32.0).shape("Hello")?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Vertical SVG output

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

## GSUB variant switching

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

## Variable font axes

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
# Ok::<(), Box<dyn std::error::Error>>(())
```

## TTC / collection access

```rust
use fontloader::FontFile;

let file = FontFile::from_file("fonts/YourCollection.ttc")?;
let face = file.face(1)?;
println!("{}", face.full_name());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Notes

- `FontEngine` is the intended place to choose shaping direction and variant behavior.
- Variable-font axis values are also configured through `FontEngine`.
- `FontOptions` still exists for lower-level control and `FontFamily` integration.
- Technical implementation notes and current limitations live in `feature-status.md`.
- CFF2 implementation notes live in `cff2-investigation.md`.
