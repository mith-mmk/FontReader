# Self-contained OpenType-SVG glyph extraction

This is a bounded passive subset, not a general SVG/CSS renderer. No public API
or dependency was added. `miniz_oxide` remains on the existing 0.7 dependency.

## Boundary

fontcore decodes the SVG table document and selects the exact `glyphN` (legacy
numeric IDs remain accepted). It indexes the XML tree, resolves transitive local
`href`/`xlink:href` and `url(#id)` references, and emits the selected subtree plus
only externally required definitions. Skeleton ancestors retain namespace declarations,
inline presentation styles, transforms, inherited paint, and viewport attributes.
Dependencies outside defs remain in their original ancestry, under a generated
defs wrapper so they do not paint as siblings. Duplicate IDs, missing selected
references, cycles, and excessive reference expansion are rejected.

The result is a complete `<svg>` document. OpenType's initial viewport uses
`head.unitsPerEm`, **not the glyph advance**. Root width/height defaults and
percentages resolve against that em square; root overflow is forced visible as
required by OpenType. Root numeric width/height and viewBox are retained; physical
CSS length units are not yet supported. Nested SVG viewports are retained.

Source: https://learn.microsoft.com/en-us/typography/opentype/spec/svg

paintcore's SVG-input route must render this document with font placement/scale.
It must not flatten opacity, nested viewport, clipping, or inherited styles into
independent paths without preserving their semantics. Existing fontcore path
commands remain available for their limited subset. For non-identity viewports
and compositing unsupported by that subset, the SVG source alternative is kept.
The existing combined Path/Svg fallback behavior is unchanged; this change does
not define a new cross-library layer-selection contract or implement paintcore's
raw SVG renderer. CompositeFont SVG serialization still rejects raw SVG layers.

## Accepted and rejected

- Accepted: passive svg/g/defs/symbol/path/basic shapes, gradients/stops,
  clipPath/mask/use, local references (including quoted url), inline presentation
  declarations, predefined/numeric XML entities, XML declaration and comments.
- SVG and XLink namespace aliases are validated and retained; the XML reserved
  namespace is checked. Legacy namespace-less SVG remains accepted.
- Rejected: DTD/custom entities, active elements, events, external URLs/images,
  CSS stylesheets/selectors, CSS escapes/comments/imports, unsupported elements
  or attributes. Non-whitespace text nodes are not supported. A whole source
  document containing unsupported semantics fails closed, even in an unselected
  sibling. This avoids silently stripping stylesheets that affect the glyph.
- Data attributes are passive and preserved. XML is case-sensitive.
- Gzip output respects DecodeLimits.max_svg_bytes; payload CRC, size, and optional
  header CRC are checked. Plain UTF-8 uses the same decoded-byte limit.
- Hard ceilings: 262,144 nodes, 128 XML/reference depth, 128 attributes per node,
  65,536 references, and 262,144 expanded-reference node cost. These protect both
  extraction and the downstream subset path conversion against expansion bombs.
- Each selected document is indexed per request. Persistent decompressed/index
  caching and a standalone high-performance SVG crate are future work, not
  claimed by this change.

## Evidence and reproduction

Unit regressions cover shared documents, exact IDs, ancestor transforms/styles,
namespace aliases/rebinding, transitive local references, unused defs, entities,
external resources, malformed trees, cyclic and exponentially expanding use
chains, gzip checksums, root clipping and named colors containing `url`.
A generated 10,000-glyph document (>700KB) extracts to <400 bytes.

Real fixtures and their separate artwork licenses live in
`tests/fixtures/svg-extraction/README.md`. Noto is a standalone SVG asset; Twemoji
is an actual decompressed OpenType SVG-table document. They are not interchangeable
font-format claims. Extracted examples are in `examples/`.

```
cargo test --all-features
FONTCORE_SVG_FONT=/path/to/TwitterColorEmoji-SVGinOT.ttf \
  cargo test --lib external_svg_font_corpus -- --ignored --nocapture
FONTCORE_SVG_EXPORT_DIR="$PWD/doc/svg-extraction/examples" \
  cargo test --lib export_real_svg_fixtures -- --ignored
python3 tools/check_svg_extraction_pixels.py
```

The external corpus check is opt-in, downloads nothing, and was run against
Twemoji Color Font v15.1.0: **3,838 glyph documents accepted, zero rejected**.
The pixel oracle requires CairoSVG and Pillow and compares all RGBA channels at
256×256. Both included real examples match their source with OpenType root
viewport normalization exactly (`pixel-check.json`). This validates extraction,
**not paintcore rendering** or conformance across all SVG features/fonts.

## Partial path conversion follow-up

The paintcore integration probe found that the existing path subset converted
only one shape of the Noto example and silently discarded unsupported path
commands in siblings. Path conversion is now atomic for reported geometry,
use-reference, element, and unresolved-paint failures: any such failure discards
all tentative paths and returns the complete raw SVG alternative. Valid
fully-converted glyphs retain their existing Path result. Intentionally empty
paint (`none`) is not treated as an unknown paint.

The Noto example now produces a raw SVG layer, not a partial face. This is a
safety correction, **not** implementation of its unsupported path commands.
paintcore's raw-SVG route remains unimplemented; its coordinated fail-closed
change must return Unsupported without painting any partial glyph. This does
not claim a complete semantic audit of all older gradient/clip/path parsing.
