# Fontloader for rust

Rust library for loading OpenType, TrueType, TTC, WOFF, and partial WOFF2 font data.

Japanese: [README.ja.md](README.ja.md)

## GlyphRun API

`src/commands.rs` now exposes `fontloader::text2commands(text, FontOptions)` for building a
`GlyphRun`.

- Pass a loaded font directly with `FontOptions::new(&font)`.
- `font_size` and `line_height` are resolved in pixels.
- `font_stretch`, `font_style`, `font_variant`, and `font_weight` are part of `FontOptions`.
- Font lookup by family or name is not implemented yet, so pass a loaded font for now.
- TrueType and CFF glyphs are returned as `GlyphLayer::Path`.
- `sbix` glyphs are returned as `GlyphLayer::Raster`.
- SVG glyph layers currently return `ErrorKind::Unsupported`.

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

`load_font`, `load_font_from_file`, and `load_font_from_buffer` are available as new aliases for
the existing `fontload*` APIs.

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
- Not implemented: `lookup_locale()`, `lookup_liga()`, `lookup_width()`, `lookup_number()`

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
