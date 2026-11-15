# Bounded GSUB filtered-context prerequisite

This is an internal, default-off API, not general GSUB filtering support or a
standalone Tibetan upper-stack positioning fix. The bounded Tibetan caller opts
in only when its normalized run contains a newly admitted stack plus one upper
vowel, and separately certifies ownership and anchors. Other presentation runs
continue to use the whole-run no-skip proof.

## API and admitted operations

The crate-private `apply_tibetan_stack_presentation_with_filtering` accepts an
explicit opt-in boolean. Passing `false` retains the existing behavior. Callers
must separately validate their shaping grammar, ownership and positioning needs;
this API does not certify those properties.

When the whole-run no-skip proof is unavailable, the opt-in path admits:

- Chaining context format 3 with contiguous included input and zero or one
  nested lookup record. Zero records constitute a successful ignore rule.
- Single substitution and contiguous ligature substitution.
- Supported extension wrappers around these operations.

Every rule is considered in font order. Backtrack and lookahead matching skip
excluded glyphs under the contextual lookup's flags. Input matching may traverse
excluded glyphs read-only to establish a definitive mismatch. A complete input
match containing a gap is unsupported and aborts before nested execution; it
cannot fall through to a later rule. An unsupported operation can be bypassed
only when the current glyph is definitively excluded or misses its first input
coverage.

Nested lookups use their own flags and the original glyph array, not a compacted
outer-context view. Child context may inspect surrounding glyphs, but child
consumption is confined to the intersection of ancestor input bounds. Multiple
records in a newly filtered context and skipped-input ligatures remain
unsupported. No new skipped-mark ligature-component ownership is introduced.

Missing required GDEF data is conservative. A valid mark-attachment class 0 is
distinct from missing class-definition data. IgnoreMarks takes precedence over
mark filtering sets, which take precedence over attachment-class filtering.

## Transactional fallback and limits

Unsupported operations, source ambiguity and exhausted limits restore the entire
opt-in presentation input, including its metadata. This prevents partial output
from escaping when a later operation fails. Existing globally proven paths keep
their supported operations; budget exhaustion there also invalidates an opt-in
transaction. The default-off path retains its previous behavior.

The existing limits remain:

- 4,096 nested lookup applications per input position.
- Nesting depth 32.
- Existing substitution expansion limit of 1,000,000 glyphs.

Newly filtered operations additionally share **65,536 work units per presentation
transaction**, without resetting at each cursor or nested lookup. Subtable and
ligature-record attempts and glyph-property probes are charged, including
coverage misses and skipped context glyphs. Globally proven legacy matching
retains its existing cost model; the new counter is not a universal bound on all
shaping work. Exhaustion uses transactional fallback, not truncated output.

## Validation

Synthetic tests cover ordered ignore rules, filtered backtrack/lookahead,
read-only input mismatch proofs, nested flags, consumption bounds, missing data,
mutation, extensions, default-off behavior and transactional limits.

An optional ignored unit test uses original Noto Serif Tibetan with SHA-256
`060ec022b04c306de3f58d051fb0e1cf81a5b610c5910fbfc43bba154c057cda`.
Set `FONTCORE_NOTO_TIBETAN` to that font and run:

```sh
cargo test --lib original_noto_opt_in_proof -- --ignored --nocapture
```

For the pinned `ཀྐི` case, opt-in presentation produces glyphs `[158, 1328]`.
Measured work for 1, 2, 8, 32, 128 and 512 repetitions is respectively 105, 224,
938, 3,794, 15,218 and 60,914 units. At 1,024 repetitions the budget is exhausted
and the exact input is restored. These are fixture-specific GSUB observations,
not general script-support or positioning guarantees.

## References

- [OpenType common layout tables](https://learn.microsoft.com/en-us/typography/opentype/spec/chapter2)
- [HarfBuzz 15.0.0 GSUB/GPOS implementation](https://github.com/harfbuzz/harfbuzz/blob/15.0.0/src/hb-ot-layout-gsubgpos.hh)
