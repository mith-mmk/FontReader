# Fontloader for Rust

Fontloader is a Rust library for loading fonts and turning text into shaped glyph runs or SVG.

Japanese: [README.ja.md](README.ja.md)

## What This Crate Exposes

- `FontFile`
  - Owns a font file or collection entry point
  - Lets you choose a face from TTC / collection data
- `FontFace`
  - Represents one face
  - Exposes simple metadata such as `family()`, `full_name()`, `weight()`, `is_italic()`
- `FontEngine`
  - Shapes text and renders output
  - Exposes `shape(text)`, `measure(text)`, and `render_svg(text)`
- `FontFamily`
  - Cache layer for multiple faces with face selection and per-glyph fallback

The low-level parser API is still available behind `features = ["raw"]`.

## Supported Formats

- TrueType
- OpenType / CFF
- TTC
- WOFF
- WOFF2

Default features include `layout` and `cff`.

## Quick Start

```rust
use fontloader::FontFile;

let file = FontFile::from_file("fonts/YourFont.ttf")?;
let face = file.current_face()?;
let engine = face
    .engine()
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

## Loading Fonts

```rust
use fontloader::{load_font_from_buffer, FontFile};

let bytes = std::fs::read("fonts/YourFont.ttf")?;

let face = load_font_from_buffer(&bytes)?;
let file = FontFile::from_buffer(&bytes)?;
assert!(file.face_count() >= 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

For TTC or collections:

```rust
use fontloader::FontFile;

let file = FontFile::from_file("fonts/YourCollection.ttc")?;
let face0 = file.face(0)?;
let face1 = file.face(1)?;

println!("{}", face0.full_name());
println!("{}", face1.full_name());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## FontFamily Cache

`FontFamily` is the high-level cache and fallback layer.

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

## Chunked WOFF2 / Range Loading

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

## Examples

Examples share a small common CLI.

- `-f`, `--font`: font path
- `-d`, `--dir`: font directory
- `-i`, `--index`: face index inside a collection
- `-o`, `--output`: output file path
- `-s`, `--string`: inline text
- `-t`, `--text-file`: text file path

High-level examples that work without `raw`:

- `api_overview`
- `fontloader`

Raw / inspection examples that require `--features raw`:

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

Run the high-level example:

```bash
cargo run --example api_overview -- -f path/to/font.ttf -s "Hello"
```

Run a raw example:

```bash
cargo run --example fonttype --features raw -- -d path/to/fonts
```

## WebAssembly

The crate compiles for `wasm32-unknown-unknown`.

- Prefer `load_font_from_buffer()` or `load_font(FontSource::Buffer(...))`
- `load_font_from_file()` and `load_font_from_net()` return `ErrorKind::Unsupported` on `wasm32`

## Raw API

If you still need the older low-level API, enable `raw`.

```toml
[dependencies]
fontloader = { version = "0.0.10", features = ["raw"] }
```

That exposes:

- `fontloader::Font`
- `fontloader::fontheader`
- `fontloader::opentype`
- deprecated compatibility aliases such as `fontload_*`

## More Detailed Notes

- Implementation notes: [doc/feature-status.md](doc/feature-status.md)
