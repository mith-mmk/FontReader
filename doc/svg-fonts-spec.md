# `svg-fonts` Specification

This document fixes the current behavior of OpenType `SVG ` support when the crate is built with `features = ["svg-fonts"]`. The implementation currently lives mainly in [../src/svgparse.rs](../src/svgparse.rs), [../src/commands.rs](../src/commands.rs), and [../src/fontengine.rs](../src/fontengine.rs).

## Goals

- Extract glyph-local SVG payloads from the OpenType `SVG ` table
- Keep those payloads as `GlyphLayer::Svg` in glyph runs
- Convert simple SVG elements into `GlyphLayer::Path` so fallback logic and simple rendering can reuse them
- Explicitly document unsupported SVG features and future extension points

## Feature Gate

- Cargo feature: `svg-fonts`
- Without `svg-fonts`, glyphs from the OpenType `SVG ` table remain unsupported through the existing non-SVG path

## Layer Model

With `svg-fonts` enabled, one SVG glyph may expose either or both of the following:

- `GlyphLayer::Svg`
  - the original glyph-local SVG fragment
  - emitted as nested SVG by `FontEngine::render_svg()` and `FontFamily::text2svg()`
- `GlyphLayer::Path`
  - a partial path conversion for simple shapes the parser understands
  - fill and stroke are represented as separate layers

`GlyphLayer::Svg` preserves source payloads. `GlyphLayer::Path` is only a partial SVG-to-path conversion and is not a complete SVG renderer.

## Node Model

The SVG parser first builds a lightweight node tree, then flattens it.

- `SvgNode`
  - `Element`
  - `Text`
- `SvgElement`
  - `name`
  - `attrs`
  - `children`

Processing is split into three stages:

1. `parse_svg_document()`
   - converts an SVG fragment into a lightweight node tree
2. `collect_definitions()`
   - collects `id`-backed elements from inside `<defs>`
3. `flatten_node()`
   - resolves inherited attributes, `use`, and transforms, then emits `PathGlyphLayer` values

## Supported Elements

Elements currently converted into `PathGlyphLayer`:

- `path`
- `rect`
- `circle`
- `ellipse`
- `line`
- `polyline`
- `polygon`

Container elements currently traversed:

- `svg`
- `g`
- `symbol`
- `defs`
- `use`

## Supported Attributes

Attributes currently interpreted:

- `fill`
- `fill-rule`
- `stroke`
- `stroke-width`
- `transform`
  - `translate(...)`
  - `scale(...)`
  - `matrix(a b c d e f)`
- `x`
- `y`
- `href`
- `xlink:href`
- `style`
  - the same properties are also read from inline style strings

### Inheritance Rules

- `fill` is inherited from the parent
- default `fill` is `currentColor`
- `fill="none"` suppresses fill-layer generation
- `stroke` is inherited from the parent
- default `stroke` is `none`
- `stroke-width` is inherited from the parent
- default `stroke-width` is `1.0`
- `fill-rule` is inherited from the parent
- default `fill-rule` is `nonzero`

### `use` Resolution

- `href="#id"` and `xlink:href="#id"` are resolved
- `x` and `y` are applied as an extra transform on the call site
- `fill`, `fill-rule`, `stroke`, and `stroke-width` from the `use` element override the referenced element

## Path Conversion Rules

### `PathGlyphLayer`

`PathGlyphLayer` now exposes these paint modes:

- `PathPaintMode::Fill`
- `PathPaintMode::Stroke`

Additional fields:

- `paint_mode`
- `stroke_width`

### Fill and Stroke Behavior

- Closed shapes may emit both a fill layer and a stroke layer
- `line` never emits a fill layer; it only emits a stroke layer when stroke paint is present
- `polyline` is currently still treated as fill-capable
  - this is a pragmatic implementation detail rather than strict SVG fidelity
  - it may be revised later

### Supported `path d` Commands

Currently supported commands:

- `M` / `m`
- `L` / `l`
- `H` / `h`
- `V` / `v`
- `Q` / `q`
- `C` / `c`
- `Z` / `z`

Currently unsupported:

- `S` / `s`
- `T` / `t`
- `A` / `a`

## SVG Export

`FontEngine::render_svg()` and `glyph_run_to_svg()` currently behave as follows:

- `GlyphLayer::Svg`
  - emitted as nested `<svg>` with the original payload
- `GlyphLayer::Path` + `PathPaintMode::Fill`
  - emitted as `<path fill="...">`
- `GlyphLayer::Path` + `PathPaintMode::Stroke`
  - emitted as `<path fill="none" stroke="..." stroke-width="...">`

Stroke-layer bounds are padded by `stroke_width / 2` during viewBox calculation.

## Unsupported or Partial Areas

The following remain unsupported or only partially covered:

- gradients
- patterns
- `clipPath`
- `mask`
- filters
- complete opacity handling
- `stroke-linecap`
- `stroke-linejoin`
- `stroke-dasharray`
- `stroke-dashoffset`
- transforms such as `rotate`, `skewX`, and `skewY`
- SVG path arcs (`A` / `a`)
- CSS class or selector-based style resolution
- external references
- full SVG-spec inheritance and presentation-attribute coverage
- actual stroke rasterization inside `paintcore`

## Non-Goals

At this stage, the feature is not trying to be:

- a full SVG renderer
- a browser-compatible CSS/DOM/SVG engine
- a complete SVG-to-command converter for every `SVG ` table payload

## Compatibility Notes

- Existing `GlyphLayer::Svg` behavior is preserved
- Simple SVG payloads may now also add `GlyphLayer::Path`
- Callers that depend on layer count or layer ordering should handle `Svg` and `Path` coexisting on the same glyph
