# API Recipes

This document keeps runnable public-API examples grouped by task.

## Read Metadata

```rust
use fontcore::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
println!("{}", face.family());
println!("{}", face.full_name());
println!("{}", face.weight().0);
println!("{}", face.is_italic());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Basic Shaping

```rust
use fontcore::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
let run = face.engine().with_font_size(32.0).shape("Hello")?;
assert!(!run.glyphs.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Measure Text

```rust
use fontcore::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
let width = face.engine().with_font_size(32.0).measure("Hello")?;
assert!(width > 0.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Vertical SVG Output

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

## Query Unicode Vertical Orientation

This scalar-property query requires no font or feature flags. The four return
values preserve Unicode's distinction between orientation and typography:

```rust
use fontcore::{
    vertical_orientation, VerticalOrientation as Vo, VERTICAL_ORIENTATION_UNICODE_VERSION,
};

assert_eq!(VERTICAL_ORIENTATION_UNICODE_VERSION, "18.0.0");
assert_eq!(vertical_orientation('漢'), Vo::Upright); // U
assert_eq!(vertical_orientation('A'), Vo::Rotated); // R
assert_eq!(vertical_orientation('、'), Vo::TransformWithUprightFallback); // Tu
assert_eq!(vertical_orientation('ー'), Vo::TransformWithRotatedFallback); // Tr
```

`U` keeps the code-chart orientation; `R` rotates it 90 degrees clockwise.
`Tu` and `Tr` prefer a typographically transformed vertical form, falling back
to upright and rotated code-chart glyphs respectively. In particular, `Tr` does
not mean "always rotate": use the transformed upright form when available.

The query uses pinned Unicode 18.0.0 data, including its unassigned and private-use
ranges. It does not resolve grapheme clusters, combining marks, variation
selectors, emoji sequences, font substitutions, or mixed vertical layout, and
does not change any shaping API. A scalar's property is not its final cluster or
glyph orientation. [Data provenance and regeneration](../data/unicode/README.md).

## Sideways LTR Shaping, Measurement, And SVG

For a single horizontal LTR line rotated clockwise as a whole, use the explicit
sideways methods. This differs from upright top-to-bottom layout.

```rust
use fontcore::{FontFile, ShapingPolicy};

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
let engine = face
    .engine()
    .with_font_size(32.0)
    .with_vertical_flow()
    .with_svg_unit("px");

let run = engine.shape_sideways_ltr("Hello")?;
let metrics = engine.measure_layout_sideways_ltr("Hello")?;
let advance = engine.measure_sideways_ltr("Hello")?;
let svg = engine.render_svg_sideways_ltr("Hello")?;

assert!(!run.glyphs.is_empty());
assert_eq!(advance, metrics.inline_advance);
assert!(svg.contains("<svg"));
assert_eq!(engine.shaping_policy(), ShapingPolicy::TopToBottom);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Each call overrides LTR, RTL, or top-to-bottom policy only for that call. The
engine and its options stay unchanged, including style, locale, variant,
supported variations, size, stretch, and SVG units. Ordinary `shape`, `measure`,
`measure_layout`, and SVG methods retain their existing behavior.

The run is shaped with horizontal metrics, then rotated 90 degrees clockwise
about baseline `(0, 0)`, without em-centering. Measurement uses the same rotated
run: `inline_advance` is the absolute sum of its y advances, `ink_bounds` encloses
the positioned ink, and `line_bounds` includes each positioned origin and its
full advance-vector endpoint. `block_extent` is the x extent of `line_bounds`,
not the font line height. Empty text has no bounds and zero extents; spaces
retain their horizontal advances rotated into the vertical direction.

Only current-color fill paths without clips are supported. Other paints
(including solid colors and gradients), strokes, clips, raster layers, or raw
SVG layers return `std::io::ErrorKind::Unsupported`, without a partial result.
TAB, CR, LF, VT, FF, NEL, U+2028, and U+2029 return
`std::io::ErrorKind::InvalidInput`. Split and position lines in the caller.
This single-face API does not add family fallback, BiDi, automatic mixed
orientation, or alignment with upright CJK baselines. See
[feature-status.md](feature-status.md#sideways-ltr-runs) for `vrtr`, coordinate,
and shaping-support details.

## RTL Shaping

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

## GSUB Variant Switching

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

## Variable Font Axes

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

## TTC Or Collection Access

```rust
use fontcore::FontFile;

let file = FontFile::from_file("fonts/YourCollection.ttc")?;
let face = file.face(1)?;
println!("{}", face.full_name());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## FontFamily Fallback

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

## Chunked WOFF2 Loading

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

## Notes

- `FontEngine` is the main place to choose shaping direction, locale, variants, and variable-font axes.
- `FontOptions` still exists for lower-level control and `FontFamily` integration.
- Current implementation notes and limitations live in [feature-status.md](feature-status.md).
- CFF2 investigation notes live in [cff2-investigation.md](cff2-investigation.md).
- For a document index, start at [README.md](README.md).

## Mixed vertical LTR shaping, measurement, and SVG

Explicit single-face, single-line LTR mixed layout. Unicode17 EGC boundaries and Unicode18 VO/Me properties are independent; see [scope and limits](feature-status.md#opt-in-mixed-vertical-ltr-outlines).

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let file = fontcore::FontFile::from_file("font.ttf")?;
let face = file.current_face()?;
let engine = face.engine().with_font_size(32.0);
let run = engine.shape_vertical_mixed_ltr("漢字AV「かな」")?;
let advance = engine.measure_vertical_mixed_ltr("漢字AV「かな」")?;
let metrics = engine.measure_layout_vertical_mixed_ltr("漢字AV「かな」")?;
let svg = engine.render_svg_vertical_mixed_ltr("漢字AV「かな」")?;
# Ok(())
# }
```
