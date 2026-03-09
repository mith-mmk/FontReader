# Fontloader for rust

Rust library for loading OpenType, TrueType, TTC, WOFF, and partial WOFF2 font data.

Japanese: [README.ja.md](README.ja.md)

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
