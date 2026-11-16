# Source-preserving shaping (additive API)

`FontEngine::shape_mapped`, `FontFamily::shape_mapped` and
`CompositeFont::shape_mapped_ltr` return `ShapedText`. Existing `shape`,
`GlyphRun`, and `PositionedGlyph` remain source-compatible. The existing rendering
path is shared, not reconstructed by repeatedly shaping prefixes.

## Contract

- The immutable result retains the complete original UTF-8 string and its
  Unicode 17 extended-grapheme byte ranges, including invisible/control text.
- `glyph_sources()` is parallel to `glyph_run().glyphs`. Each range is an
  original UTF-8 **conservative contiguous ownership envelope**. Normalization
  and decomposition can give several glyphs the same original grapheme. A
  ligature merges component ownership; intervening ignored marks may therefore
  overlap the envelope. This is many-to-many ownership, not exact scalar causality.
- A variation selector belongs to the original grapheme. Characters omitted by
  existing shaping are retained in `text()`/`grapheme_ranges()` but need not own a
  glyph. No replacement glyph is fabricated to make the map total.
- Face IDs and shaping-segment IDs are local to the result, not persistent font
  identifiers. Single-face engines use zero. Family IDs refer to cached face
  insertion indices; composite IDs are documented by its method.
- Explicit direction is inherited from the existing shaping engine. This does
  not add Unicode paragraph BiDi, line breaking, selection rectangles, or visual
  keyboard movement. Mixed vertical/sideways APIs are not mapped by this change.
- Byte offsets are not JavaScript UTF-16 offsets. UI adapters must convert using
  the original text, never glyph counts or normalized strings.

## Boundaries and next stages

Source ownership is a prerequisite for editing but not a complete editor model.
Do not use separate prefix measurement as an oracle for caret placement: it can
change kerning, joining, and ligatures. Caret geometry must use the shaped run.

Mapped results expose optional font-authored ligature caret candidates through
`glyph_ligature_carets()`. Static GDEF Format 1 and Format 3 without adjustment
are supported; contour points, Device/VariationIndex and variable faces fall
back conservatively. Layout adopts candidates only with unambiguous single-glyph
ownership, matching EGC count, and finite strictly interior ordered positions.
`CaretSource::FontAuthored` distinguishes adoption from `Approximate` fallback.
The legacy `CaretQuality` remains conservative and unchanged for compatibility.

Paragraph BiDi needs resolved embedding levels, visual run ordering and affinity
at directional boundaries. Wrapping needs line ownership and line boxes. Mixed
vertical orientation needs per-run transforms. These belong in a higher text
layout object, with UI movement/deletion/IME policy above it; ink bounds alone
cannot supply them. Existing paintcore adapters discard glyph metadata and need
an explicit mapped-result adapter before canvas editing can consume this API.

## Bounded caret and hit-test API

`CaretLayout::new(&shaped)` builds a single-line, explicitly directed layout.
`caret_positions(TextPosition::new(byte, affinity))` accepts only original EGC
boundaries; `hit_test(x, y)` selects the nearest inline stop, clamps outside
points, and ignores the cross-axis coordinate. Ties choose the lower logical byte
offset. Both affinities currently share geometry. LTR uses positive x, explicit
RTL negative x, and upright TTB positive y. Geometry is pen/advance based, not ink
or GPOS mark placement. Composite baseline shifts are also ignored: returned
points stay on the common initial pen baseline, not the shifted glyph baseline.
They do not specify caret height or selection rectangles. Internal stops prefer validated GDEF candidates; otherwise they use equal
advance division. Both retain legacy `CaretQuality::Approximate`; inspect
`source()` for the detailed provenance.

This deliberately rejects tabs/newlines/BiDi controls, unowned source gaps,
reordered ownership, non-finite/negative inline advances and nonzero cross-axis
advances with `Unsupported`. Non-finite hit-test coordinates and non-EGC byte
positions return `InvalidInput`. Empty text has a single origin stop. This is not
mixed-direction paragraph hit testing.

Stage 3 remains a separate layout project: returning guessed positions for
paragraph BiDi or wrapped/mixed-vertical text would silently violate the source
and geometry contract above. The current shaping result supplies ownership but
not paragraph levels, line boxes, or mixed-orientation transforms. No consumer
such as paintcore is modified in this change.

```rust,no_run
use fontcore::{FontFile, CaretLayout, TextPosition, CaretAffinity};
let file = FontFile::from_file("font.ttf")?;
let face = file.current_face()?;
let shaped = face.engine().shape_mapped("office")?;
let layout = CaretLayout::new(&shaped)?;
let caret = layout.caret_positions(TextPosition::new(2, CaretAffinity::Downstream))?;
let nearest = layout.hit_test(20.0, 0.0)?;
# Ok::<(), std::io::Error>(())
```
