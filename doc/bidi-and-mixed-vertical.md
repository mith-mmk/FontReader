# Bounded paragraph BiDi and mapped mixed vertical layout

These additions extend the hard-line branch. They do not change legacy `shape`
behavior, introduce automatic wrapping, or make all OpenType scripts supported.

## Data and shaping order

The new paragraph route pins `unicode-bidi = 0.3.18` and
`unicode-bidi-mirroring = 0.4.0`, both using Unicode 16.0 data. Existing grapheme
segmentation uses Unicode 17.0 and vertical orientation uses Unicode 18.0. These
versions are deliberately explicit; this is not a claim of uniform Unicode 18
conformance. The dependencies are MIT/Apache-2.0 licensed.

BiDi levels are resolved on the original UTF-8 input before rendering controls
are removed. Visual run ranges refer to original bytes, and each run is shaped
in logical order. RTL rendering moves the pen toward negative x, so the run is
translated to the right edge of its allocated visual span, not reversed twice.

Mirroring replaces the internal shaping scalar before nominal cmap lookup while
retaining original source ownership. It never rewrites the public original text.
Characters with mirroring property but no mapped counterpart are rejected rather
than falsely treated as mirrored. This scalar-based policy is not font-specific
`rtlm` substitution. BiDi formatting controls produce no glyph/advance in this new
route, even if the font supplies a visible glyph for them. ZWJ/ZWNJ are retained
for joining. Legacy rendering keeps its original policies.

Directional overrides are not silently treated as supported: Arabic joining is
still coupled to RTL in the legacy engine. A general implementation requires
joining decisions independent of paragraph overrides. Existing script-shaping
limits remain, and run boundaries that split an extended grapheme are rejected.

## Mapped mixed vertical geometry

`FontEngine::shape_vertical_mixed_ltr_mapped` retains the exact existing explicit
LTR mixed-orientation rendering policy. Each group exposes its original byte
range, local mapped glyphs and actual rotation/translation. Sideways groups use
horizontal shaping with `vrtr`, clockwise rotation and the existing centering
shift; upright groups use vertical shaping with `vert`.

Caret points follow the same transforms as rendering. At an orientation boundary,
upstream/downstream affinity selects the preceding/following group, which may
have different cross-axis points. No ink-derived caret height is implied.
Drawing metadata is available independently of optional caret support: mappings
unsupported by the bounded local caret model return an error when caret geometry
is requested rather than corrupting the render result.

This is still single-face, single-line, explicit LTR group classification.
Mixed-direction sideways runs, paragraph BiDi combined with vertical orientation,
font-family fallback and automatic wrapping are separate work.

Sources: [unicode-bidi](https://docs.rs/unicode-bidi/0.3.18/unicode_bidi/),
[mirroring](https://docs.rs/unicode-bidi-mirroring/0.4.0/unicode_bidi_mirroring/),
[Unicode UAX #9](https://www.unicode.org/reports/tr9/).

## Paragraph API and editing contract

`BidiParagraphLayout::shape(&engine, text, BidiBaseDirection::Auto)` resolves
horizontal hard lines with first-strong base direction; explicit LeftToRight or
RightToLeft selects a fixed base level. The engine remains single-face. Each
`BidiLine` retains original content/break ranges and exposes visual runs, their
embedding levels, local `ShapedText`, and rendering translations.

`visual_carets(byte)` exposes all sides of an original EGC boundary. The same
logical position can have different visual x coordinates at a direction change.
`caret_positions(TextPosition)` selects upstream/downstream affinity. Formatting
controls keep zero-width source stops; a control swallowed inside a ligature
ownership envelope does not get a share of its advance. Ligature-internal stops
prefer validated static GDEF candidates and otherwise remain explicitly approximate;
`source()` reports the distinction while legacy `quality()` remains conservative. `hit_test` selects nearest baseline then inline
stop, with deterministic logical-byte/affinity ties and finite-extreme clamping.

LF/CRLF are hard paragraph boundaries; empty/trailing lines are retained. There
is no soft wrapping, tab expansion, glyph-ink containment or inferred line box.
Unsupported overrides, vertical paragraph mode, source reordering/gaps other
than supported BiDi controls, partial EGC level runs, or invalid geometry return
an error rather than partial layout.

Isolate initiators/PDI remain shaping barriers after level resolution: unlike
transparent direction marks, removing them must not join Arabic or form a
ligature across an isolate boundary. They emit no glyph or advance but terminate
GSUB/GPOS context. This follows the distinction in UAX #9 §3.5.

### Joining across visual level runs

With `layout` enabled, paragraph construction compares Arabic-style joining forms
for the full logical line against forms for each resolved run. If splitting the
line would lose joining context, construction returns `Unsupported`. For example,
Arabic letters separated by LRM can become separate level runs while LRM remains
transparent for joining; independently shaping those runs would incorrectly
produce isolated forms. RLM/ALM that remain in the same run and isolate controls
that intentionally break joining retain their supported behavior. This guard is
not a contextual cross-run shaper. Without `layout`, contextual joining is absent
as in the existing engine, so this feature-dependent guarantee does not apply.
