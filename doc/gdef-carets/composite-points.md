# Bounded composite Format 2 carets

The next Format 2 stage supports static monochrome TrueType composite glyphs
only when their root xMin equals hmtx LSB and every component uses integer XY
translation. Scale/matrix transforms, point attachment, USE_MY_METRICS and
component instructions are rejected rather than approximated as authored data.
Variable and COLR point resolution remain outside this path.

The point decoder flattens component points in source component order. Child
LSB shifts are not applied inside the parent. Nested traversal rejects cycles
and has depth 4, component count 32 and total decoded point count 4096 limits.
All decoded leaves and transformed points retain the legacy renderer arithmetic
eligibility checks. Existing limits on the number of point carets also apply.
Failure rejects the authored candidate set, leaving the explicit approximation
policy unchanged. This does not harden or expand the legacy outline renderer.

The first two faces of the original MIT fixture `gdef-composite-carets.ttc`
exercise translated and nested translated components. Independent FreeType-backed
HarfBuzz queries give X=470/490 and TTB down-distance=725 for both. Deliberately
differing child LSBs prove that adding child metric-origin corrections would be
wrong. Unsupported transforms, root-bearing mismatch, metrics inheritance and
malformed point references remain negative controls. See the [oracle details](point-carets-reference.md#bounded-translated-composite-extension).

The remaining general composite and variable cases need separate semantics and
reference coverage. This stage does not claim all composite TrueType point
indices or arbitrary transformed outlines supported.
