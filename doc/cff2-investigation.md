# CFF2 Investigation

## Goal

Before implementing CFF2 outline execution, verify how much of the current `src/opentype/outline/cff.rs` can be shared with a future CFF2 path.

## Shareable layers

These parts are format-agnostic enough to be shared between CFF and CFF2:

- INDEX parsing
- DICT parsing
- operand decoding
- charstring token decoding
- subroutine bias calculation and subroutine dispatch
- path command emission

In practice, the reusable core is "compact font bytecode decoding", not the whole current `CFF::new()` loader.

## CFF1-specific layers

The following logic is specific to CFF/CFF1 and should stay outside a shared core:

- Name INDEX and String INDEX loading assumptions
- Top DICT operators used only by CFF1
- Encodings / charsets / FDSelect / FDArray wiring as currently loaded
- width and default-width behavior currently derived from Private DICT
- CFF1-only top-level table layout assumptions

## CFF2-specific layers

CFF2 needs extra logic that does not fit cleanly into the current `CFF::new()` flow:

- CFF2 top dict differences
- variation store parsing
- `vsindex`
- `blend`
- glyph variation evaluation against normalized coordinates
- CFF2 charstring execution rules

## Recommended refactor

The clean split is:

1. `cff_shared`
   - INDEX / DICT / operand / charstring decoding
   - shared subroutine execution helpers
   - shared path building
2. `cff1`
   - current CFF font loading
   - current Private DICT width handling
3. `cff2`
   - variation store
   - `vsindex` / `blend`
   - normalized coordinate application

## Current conclusion

Yes, CFF and CFF2 can share a meaningful internal library, but not the current monolithic loader as-is.

The next safe step is to extract the shared bytecode and dictionary machinery first, then add the CFF2-specific variation path on top of that split.
