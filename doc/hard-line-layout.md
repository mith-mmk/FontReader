# Hard-line layout (stage 3a)

`HardLineLayout::shape(&engine, text)` builds an upper layer on mapped shaping
and bounded caret geometry. `lines()` exposes each line’s local `ShapedText`,
original content/break ranges, and the translation required to render it.
`caret_positions` and `hit_test` return global UTF-8 positions and layout points.
It is intentionally separate from paragraph BiDi or automatic wrapping.

## Supported boundary

- One selected `FontEngine` face, with its explicit LTR, RTL or upright TTB flow.
- LF and CRLF split hard lines. Original UTF-8 offsets retain the break bytes.
- Fixed line advance uses the engine's `line_height`, or `font_size` when absent.
- Horizontal lines advance downward; TTB columns advance leftward.
- Caret points remain on each line's initial pen baseline. Hit testing first
  chooses the nearest baseline on the block axis, then the nearest inline stop.
  This does not infer typographic line boxes from ink or select overlapping glyphs.
- Empty and trailing lines are real editable lines. The boundary inside CRLF is
  not editable, while the before-break and after-break positions remain distinct.
- A bare CR, tabs, paragraph BiDi controls, mixed vertical orientation and
  unsupported single-line caret mappings remain explicit errors.

## Still separate work

Paragraph BiDi must resolve embedding levels and visual order before shaping the
corresponding logical runs. `unicode-bidi::BidiInfo::visual_runs` is a candidate
for byte-indexed UAX #9 levels/order, but does not by itself solve font shaping,
combining-mark placement (L3), mirrored glyphs (L4), invisible control ownership,
or dual caret positions at directional boundaries. Its Unicode data version must
also be reconciled with the library's pinned segmentation/orientation data.

Mixed vertical orientation must retain each group's transform and source mapping
through sideways rotation and upright placement. Reusing a TTB direction flag
without those transforms is not a correct adapter.

Sources: [unicode-bidi BidiInfo](https://docs.rs/unicode-bidi/latest/unicode_bidi/struct.BidiInfo.html),
[Unicode UAX #9](https://www.unicode.org/reports/tr9/).

### Concrete existing shaping constraints

- RTL keeps logical glyph order and advances the pen toward negative x. A visual
  RTL run would need a translation to its right edge, not string/glyph reversal.
- Nominal cmap resolution has no BiDi mirrored-scalar override and the current
  sequence feature stages do not apply `rtlm`/`rtla`.
- ALM/LRM/RLM and embedding/isolate controls can currently enter nominal glyph
  resolution. Removing them before BiDi analysis loses required context; keeping
  them as drawn `.notdef` glyphs is equally incorrect.
- Arabic joining forms are selected only for the existing RTL shaping mode.
  Directional overrides therefore need joining behavior independent of run
  direction before general BiDi can be claimed.
- Directional boundaries need multiple visual caret positions for one logical
  byte position. A one-position-per-byte model cannot encode that ambiguity.
