# Fontloader for Rust

Fontloader is a Rust library for loading fonts, selecting a face, shaping text, and exporting SVG.

Japanese: [README.ja.md](README.ja.md)

## What To Use First

Most users only need these three types.

- `FontFile`
  - Opens a font file, TTC, or in-memory buffer
  - Lets you choose a face
- `FontFace`
  - Represents one face
  - Exposes metadata such as `family()`, `full_name()`, `weight()`, `is_italic()`
- `FontEngine`
  - Shapes text, measures text, and renders SVG
  - Main entry point for direction, locale, variant, and variable-font axes

The old low-level parser surface still exists behind `features = ["raw"]`.

## Supported Formats

- TrueType
- OpenType / CFF
- TTC
- WOFF
- WOFF2

Default features include `layout` and `cff`.

## Features

- `layout`
  - GSUB / GPOS shaping
  - Vertical flow, RTL, locale, and variant selection
- `cff`
  - OpenType / CFF outlines
- `raw`
  - Legacy low-level parser API
- `svg-fonts`
  - Provisional support for OpenType `SVG ` glyphs through `GlyphLayer::Svg`
  - Currently regression-tested mainly against `EmojiOneColor.otf` and `NotoColorEmoji-Regular.ttf`
  - `FontEngine::render_svg()` and `FontFamily::text2svg()` emit nested SVG fragments
  - Full path conversion and CSS / text interpretation are not implemented yet

## Install

```toml
[dependencies]
fontloader = "0.0.10"
```

If you need the low-level parser API:

```toml
[dependencies]
fontloader = { version = "0.0.10", features = ["raw"] }
```

If you also want provisional SVG emoji font support:

```toml
[dependencies]
fontloader = { version = "0.0.10", features = ["svg-fonts"] }
```

## Quick Start

```rust
use fontloader::{FontFile, ShapingPolicy};

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

## Common Tasks

- Show metadata
  - `face.family()`
  - `face.full_name()`
  - `face.weight()`
  - `face.is_italic()`
- Shape text
  - `engine.shape(text)`
- Measure text
  - `engine.measure(text)`
- Render SVG
  - `engine.render_svg(text)`
- Vertical flow
  - `engine.with_vertical_flow()`
- RTL shaping
  - `engine.with_right_to_left()`
- GSUB variant selection
  - `engine.with_font_variant(...)`
- Variable-font axes
  - `face.variation_axes()`
  - `engine.with_variation("wght", 700.0)`

More runnable examples live in [doc/api-recipes.md](doc/api-recipes.md).

## About SVG Color Fonts

`sbix` is exposed as raster layers, `COLR/CPAL` as path layers, and the OpenType `SVG ` table as `Svg` layers only when `svg-fonts` is enabled.

The current `svg-fonts` implementation still preserves glyph-local SVG payloads, but it now also converts simple `path`, `rect`, `circle`, `ellipse`, `line`, `polyline`, and `polygon` elements into `PathGlyphLayer` values. It includes minimal `defs` / `use`, `fill` / `fill-rule` / `stroke` / `stroke-width`, and `translate` / `scale` / `matrix` support.

See [doc/svg-fonts-spec.md](doc/svg-fonts-spec.md) for the exact current scope and limitations.

## Examples

High-level examples that work without `raw`:

- `api_overview`
- `fontmetadata`
- `fontloader`

Low-level inspection examples that require `--features raw`:

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

Shared CLI flags:

- `-f`, `--font`: font path
- `-d`, `--dir`: font directory
- `-i`, `--index`: face index inside a collection
- `-o`, `--output`: output file path
- `-s`, `--string`: inline text
- `-t`, `--text-file`: text file path
- `--vertical`: top-to-bottom flow
- `--variant`: variant shortcut such as `jp78`, `jp90`, `trad`, `nlck`

Run a high-level example:

```bash
cargo run --example api_overview -- -f path/to/font.ttf -s "Hello"
```

Run a raw example:

```bash
cargo run --example fontheader --features raw -- -f path/to/font.otf
```

## `cargo doc`

The crate has rustdoc on the public API surface.

```bash
cargo doc --no-deps
```

If you want to open it immediately:

```bash
cargo doc --no-deps --open
```

The crate-level docs and type docs are the best entry point for:

- `FontFile`
- `FontFace`
- `FontEngine`
- `FontFamily`
- `FontVariant`
- `ShapingPolicy`

## WebAssembly

The crate supports `wasm32-unknown-unknown`.

- Prefer `load_font_from_buffer()` or `load_font(FontSource::Buffer(...))`
- `load_font_from_file()` and `load_font_from_net()` return `ErrorKind::Unsupported` on `wasm32`

## Documentation Map

- Overview of the extra docs: [doc/README.md](doc/README.md)
- Public API recipes: [doc/api-recipes.md](doc/api-recipes.md)
- Current implementation status and limitations: [doc/feature-status.md](doc/feature-status.md)
- CFF2 investigation notes: [doc/cff2-investigation.md](doc/cff2-investigation.md)
