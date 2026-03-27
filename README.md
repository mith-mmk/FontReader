# Fontloader for rust

Rust library for loading OpenType, TrueType, TTC, WOFF, and partial WOFF2 font data.

Japanese: [README.ja.md](README.ja.md)

## GlyphRun API

`src/commands.rs` now exposes `fontloader::text2commands(text, FontOptions)` for building a
`GlyphRun`.

- Pass a loaded font directly with `FontOptions::new(&font)`.
- `font_size` and `line_height` are resolved in pixels.
- `font_stretch`, `font_style`, `font_variant`, and `font_weight` are part of `FontOptions`.
- `FontOptions::with_locale("ja-JP")` can request GSUB `locl` substitutions when the `layout` feature is enabled.
- `FontOptions::from_family(&family)` can resolve a cached `FontFamily` entry by family/name/weight/style/stretch.
- `FontFamily` matching is currently cache-based. Fallback chains and Last Resort selection are not implemented yet.
- `FontFamily` now exposes `text2svg()`, `text2commands()`, `text2glyph_run()`, `measure()`, and `options()` as the higher-level family entrypoint.
- TrueType and CFF glyphs are returned as `GlyphLayer::Path`.
- `sbix` glyphs are returned as `GlyphLayer::Raster`.
- COLR/CPAL colors are carried in `GlyphPaint::Solid(0xAARRGGBB)` so they can be passed directly to `paintcore::path::draw_glyphs`.
- SVG glyph layers currently return `ErrorKind::Unsupported`.
- The legacy `font.text2command()` API only returns outline commands and does not carry per-layer paint. It is now deprecated. Use `fontloader::text2commands(..., FontOptions)`, `LoadedFont::text2glyph_run()`, or `FontFamily::text2glyph_run()` when you need color glyph data.

## Renderer Integration

When connecting `fontloader` to a renderer such as `paintcore::path::draw_glyphs`, use the
`GlyphRun` API rather than the legacy outline-only API.

- Use `fontloader::text2commands(text, FontOptions)`, `LoadedFont::text2glyph_run()`, or `FontFamily::text2glyph_run()`.
- `GlyphPaint::Solid(u32)` uses packed `0xAARRGGBB`.
- `GlyphPaint::CurrentColor` means "use the default color passed into the renderer".
- COLR/CPAL glyphs keep their per-layer colors in `GlyphPaint::Solid(...)`.
- `sbix` glyphs are emitted as `GlyphLayer::Raster`.
- `font.text2command()` and `font.text2commands()` return `Vec<GlyphCommands>` for legacy
  outline workflows only. They do not preserve layer paint, raster glyph payloads, or color font
  information.

In short:

- Color-aware rendering: `GlyphRun`
- Outline-only compatibility: `GlyphCommands`

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

`load_font`, `load_font_from_file`, and `load_font_from_buffer` are the preferred loader APIs.
The old `fontload*` aliases remain for compatibility but are deprecated.

## Chunked font loading

For parallel or range-based downloads, use `ChunkedFontBuffer` to rebuild a complete font buffer
before decoding it.

- This is especially useful for WOFF2 delivery split into multiple byte ranges.
- The current WOFF2 path still requires the complete byte stream before decode.
- `append(offset, bytes)` accepts chunks in any order.
- `missing_ranges()` reports which byte ranges still need to be fetched.
- `into_loaded_font()` and `load_font()` hand the reconstructed bytes to the existing loader.

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

## FontFamily cache

`FontFamily` sits on top of loaded fonts and `ChunkedFontBuffer`.

- Register a fully loaded face with `add_loaded_font()` or `add_face(...)`.
- Register an in-flight face with `begin_chunked_face(face_id, descriptor, total_size)`.
- Feed chunks in any order with `append_chunk(face_id, offset, bytes)`.
- Inspect `missing_ranges(face_id)` when you need more byte ranges.
- Promote the finished face into the cache with `finalize_chunked_face(face_id)`.
- Resolve a face during shaping with `FontOptions::from_family(&family)` plus
  `with_font_family(...)`, `with_font_name(...)`, `with_font_weight(...)`,
  `with_font_style(...)`, and `with_font_stretch(...)`.
- For direct use, `family.options()` returns `FontOptions` already anchored to the family.
- `family.text2svg(...)` and `family.measure(...)` use the best cached face for the family with default matching.

This is meant for parallel fetch / reassembly first. It is not a true lazy WOFF2 decoder.

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

let run = family.text2commands(
    "Hello",
    family.options().with_font_weight(FontWeight::BLACK),
)?;
assert!(!run.glyphs.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## WebAssembly

The library now compiles for `wasm32-unknown-unknown`.

- Prefer `load_font_from_buffer()` or `load_font(FontSource::Buffer(...))` on WebAssembly.
- `load_font_from_file()` and `load_font_from_net()` return `ErrorKind::Unsupported` on
  `wasm32`.

## Layout support status

`layout` feature is partially implemented.

### GSUB

- Parsed: ScriptList, FeatureList, LookupList
- Implemented: `lookup_vertical()` for single substitution based vertical forms
- Partial: `lookup_ccmp()` exists but does not expand results yet
- Implemented: `lookup_locale()` and `lookup_liga()`
- Text APIs: `text2command()`, `text2commands()`, and `measure()` apply variation selectors and basic `locl` / `liga` / `dlig` shaping
- Not implemented: `lookup_width()`, `lookup_number()`

### Lookup parsing

- Type 1 Single Substitution: parsed and expandable
- Type 2 Multiple Substitution: parsed and expandable
- Type 3 Alternate Substitution: parsed and expandable
- Type 4 Ligature Substitution: parsed and expandable
- Type 5 Context Substitution:
  Format 1 is expandable
  Format 2 and Format 3 are parsed but not fully applied
- Type 6 Chaining Context Substitution:
  Format 1 is expandable
  Format 2 is only partially applied
  Format 3 is parsed but not applied
- Type 7 Extension Substitution: parsed, not applied
- Type 8 Reverse Chaining Contextual Single Substitution: parsed, not applied

### GDEF

- Parsed: glyph class definitions, attach list, ligature caret list, mark attach class definition, mark glyph sets definition
- Current state: data is loaded and printable for inspection, but not yet integrated into higher level shaping behavior

## Running examples

Examples are under `examples/`.

Basic run:

```bash
cargo run --example fontloader -- path/to/font.ttf
```

Examples that need layout parsing:

```bash
cargo run --features layout --example fontgsub -- path/to/font.ttf
```

Examples that need CFF support:

```bash
cargo run --features cff --example fontsvg -- path/to/font.otf
```

You can also combine features:

```bash
cargo run --features full --example fontgsub -- path/to/font.otf
```

If the font path is omitted, some examples try to use a platform default font.
