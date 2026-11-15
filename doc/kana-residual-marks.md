# Bounded residual kana advance fallback

With `layout`, glyph runs and legacy raw text export suppress a residual
U+3099/U+309A nominal advance only for an original extended grapheme containing
exactly one kana letter and one of those marks. The admitted bases are
U+3041–3096, U+309D–309F, U+30A1–30FA, U+30FD–30FF and U+31F0–31FF.
This does not compose Ainu sequences or invent an anchor. The mark retains its
nominal cursor position and its own vertical-origin translation.

The first stage is deliberately conservative:

- The face must have GPOS but no GDEF GlyphClassDef. An existing class table,
  even if it does not classify this mark, disables this fallback.
- The mark must be renderable, untouched by GSUB (including same-ID
  substitutions), and not a ligature or expansion output. Its original base
  must survive immediately before it as a single output, possibly a vertical
  or other single substitution. A missing base does not make the mark missing.
- Effective selected SinglePos or PairPos coverage disables the fallback,
  including zero-valued records and extension lookups. Font-authored anchor
  positioning retains the existing attachment path.
- Standalone marks, repeated marks, selectors, joiners, other combining marks,
  spacing U+309B/U+309C, no-GPOS faces, and marks restored by failed canonical
  composition retain the existing behavior. Without `layout`, nothing changes.

Hiragino Sans W3 has nominal 1000-unit advances for these marks, no GDEF, and
no selected positioning for them. Residual pairs previously consumed an extra
cell. Moving the following glyph into that vacated cell can leave the mark
touching or overlapping the following glyph (observed with `あ゙い`). This patch
does not fix attachment or ink overlap. Noto Sans CJK JP uses an authored GDEF and vertical SinglePos and is
outside this fallback. These are observed fonts, not a general guarantee of
Japanese shaping, mark aesthetics, or browser parity.

HarfBuzz 15's default shaper synthesizes mark classes only in the absence of
GDEF classes and zeroes mark advances late, after GPOS. That also erases an
explicit authored advance. This implementation intentionally does **not** copy
that policy: authored positioning is preserved, as are out-of-scope cases.

Primary reference: [HarfBuzz 15 shaping plan and zeroing](https://github.com/harfbuzz/harfbuzz/blob/15.0.0/src/hb-ot-shape.cc).
Regression fixtures are original MIT data documented in `tests/fixtures/README.md`.
