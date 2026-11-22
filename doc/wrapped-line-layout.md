# Bounded soft-wrap and multi-face line layout (stage 3c)

`WrappedLineLayout` builds greedy soft line wrapping on top of mapped shaping
and `CaretLayout`, alongside `FontFamily` and `CompositeFont` entry points on
both `HardLineLayout` and `WrappedLineLayout`:

- `HardLineLayout::shape_family(&family, text, options)`
- `HardLineLayout::shape_composite_ltr(&composite, text)`
- `WrappedLineLayout::shape(&engine, text, max_inline_extent, wrap_mode)`
- `WrappedLineLayout::shape_family(&family, text, options, max_inline_extent, wrap_mode)`
- `WrappedLineLayout::shape_composite_ltr(&composite, text, max_inline_extent, wrap_mode)`

## Contract

- Hard paragraphs are split by LF and CRLF, retaining empty and trailing lines
  and rejecting bare CR, tabs, other control characters, and BiDi formatting
  controls.
- `max_inline_extent` and `line_height` must be positive and finite.
- Each soft-wrapped visual line is re-shaped as its own slice so ligatures,
  kerning, and contextual substitutions are never split mid-glyph across visual
  lines.
- `WrapMode::WordOrGrapheme` prefers breaking after ASCII space (`' '`) or
  ideographic space (`'\u{3000}'`) when a break fits, falling back to Unicode 17
  extended grapheme cluster boundaries. `WrapMode::Grapheme` breaks at any
  extended grapheme cluster boundary.
- When a single extended grapheme cluster exceeds `max_inline_extent`, that
  single cluster is emitted on its own visual line to guarantee forward
  progress without dropping bytes.
- Soft-wrapped lines partition each hard paragraph contiguously with no dropped
  whitespace bytes. At a soft-wrap boundary, `visual_carets(byte)` returns both
  sides: the end of the preceding visual line (`CaretAffinity::Upstream`) and
  the start of the continuation line (`CaretAffinity::Downstream`).
  `caret_positions(TextPosition)` selects the side by affinity, and `hit_test`
  returns matching affinity so round-tripping stays on the same visual line.

## Explicit boundaries

- This is not full Unicode UAX #14 line breaking, Kinsoku/hyphenation, text
  alignment/justification, or paragraph BiDi wrapping.
- `CompositeFont` multi-line helpers remain horizontal LTR, matching
  `CompositeFont::shape_mapped_ltr`.
