# Extension parser validation

GSUB lookup type 7 and GPOS lookup type 9 are wrappers around one other
lookup subtable. OpenType requires format 1 and prohibits an extension
from referring to the same extension lookup type. Validate these fields
before dispatching to the target parser, so a nested or self-referencing
extension cannot recurse during font loading. Reject unknown target types
and NULL extension offsets, and use checked arithmetic for the relative
32-bit offset. These deliberate malformed-input rejections return
`std::io::ErrorKind::InvalidData`; underlying reader errors still propagate.

Valid GSUB types 1–6 and 8 and GPOS types 1–8 remain accepted. GPOS types
3, 7 and 8 retain their existing `Unsupported` result: their target bodies
are not parsed or range-validated. Other supported target bodies retain
their existing parser validation. There is no arbitrary nesting or offset
limit, and no requirement that an offset be at least the header size.

This is not comprehensive OpenType sanitization. Enforcing equal target
types across all extension subtables of one lookup remains outside these
checks. Runtime GSUB context recursion guards are unchanged.

Declared-table containment is enforced separately at the SFNT GSUB/GPOS
loader dispatch, not by the Extension header guard. The loader checks the
configured table-size limit and physical range before reading exactly the
declared bytes into an owned, temporary reader. Nested reads therefore cannot
consume another table or trailing container bytes. WOFF2 inherits this path
after decoding to SFNT; WOFF1 already supplies isolated table readers. Shared
TTC table bodies remain supported, and layout-disabled loading still skips
GSUB/GPOS bodies. Other OpenType table readers are unchanged.

Reader failures retain their underlying error kinds. Optional FeatureVariations
parse errors and errors in later TTC faces retain their existing suppression;
unsupported GPOS target bodies remain unparsed. Containment does not promise
that every malformed layout table is rejected. This addresses malformed-font
containment, not demonstrated memory corruption.

Original deterministic byte fixtures test invalid formats and target types,
self/nested wrappers, NULL/out-of-range offsets, header/body truncation,
valid GSUB single/multiple/ligature/reverse-chain targets and GPOS single
positioning. Valid targets are compared at both ordinary and greater-than-
16-bit offsets with a nonzero base. Existing PairPos extension and real-font
regressions complement these tests. The regression was first demonstrated
with safe invalid-format assertions only; recursive baseline inputs were
not executed.

Sources: [GSUB type 7](https://learn.microsoft.com/en-us/typography/opentype/spec/gsub#lookup-type-7-subtable-substitution-subtable-extension)
and [GPOS type 9](https://learn.microsoft.com/en-us/typography/opentype/spec/gpos#lookup-type-9-subtable-positioning-subtable-extension).

Declared-length regression tests use original synthetic fonts with backing
bytes outside the advertised table still present. They cover truncated headers,
list/Extension targets outside the table, partial leaf reads, exact-end success,
large valid Extension offsets, shared TTC tables, and WOFF1 equivalence. See
[OpenType font-file table lengths](https://learn.microsoft.com/en-us/typography/opentype/spec/otff).
