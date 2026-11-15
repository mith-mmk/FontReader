# Scalar localized substitution safety

The scalar GSUB helper used by `locl` and Arabic joining forms has only one
input glyph. It must not select a type 4 ligature that requires additional
components. Previously it returned the first covered ligature without checking
its components, including inside type 7 ExtensionSubstitution.

The helper now scans records in order for an exactly one-input ligature:
`component_count == 1` and an empty remaining-component list. Other records
are skipped; later records, subtables and lookups remain eligible. Rejected
records do not mutate source ownership or substitution flags. Valid identity
substitutions retain existing conservative ambiguity flags. Single, Alternate
and the existing first-output Multiple behavior are unchanged. This does not
add sequence `locl`, Indic feature staging or script/language-tag mapping.

## Reproduction and evidence

Noto Sans Devanagari Regular 2.007 (Google Fonts, OFL-1.1), default axes,
SHA-256 `14ec4af41f27482216d1c2229f417ff9b1425e1babb014e57d1d40d03229853e`:

- Input U+0915, fontcore locale `san`: previously gid 329 / advance 766,
  although the selected SAN ligature also requires `uni094D0930`.
- After the fix, `sa`, `san` and `sa-Deva` each produce gid 56 / advance 768,
  matching HarfBuzz 15.0.0 OT, script Deva, LTR, default features, default axes.
- Public outlines and geometry were compared using an external diagnostic
  glyph fingerprint map. Original font bytes were used for actual layout;
  diagnostic copies were not production inputs. All matched IDs were unique.
- All 264 comparisons (22 selected cases, 4 local fonts, 3 locales) equal
  baseline `sa` public output. This removes 22 Noto `san` corruptions, not the
  remaining Indic shaping gaps. Three local proprietary faces were not copied
  into this repository. No GUI or Chrome parity is claimed.

Geometry was rescaled by the engine's measured font scale, not nominal
size/unitsPerEm. HarfBuzz clusters use Unicode scalar indices with
MONOTONE_GRAPHEMES; public fontcore outputs do not expose source intervals.

Synthetic tests use original MIT outlines only. They cover direct/Extension
ligatures, ordered fallback, valid one-input and identity records, malformed
component metadata, Single/Alternate/legacy Multiple behavior, Arabic caller
forms, family rendering, raw helper behavior and no-layout controls. Existing
sequence `liga` remains able to process its complete input.

References:
- https://learn.microsoft.com/en-us/typography/opentype/spec/gsub#lookuptype-4-ligature-substitution-subtable
- https://learn.microsoft.com/en-us/typography/script-development/devanagari
- https://github.com/harfbuzz/harfbuzz/blob/15.0.0/src/hb-ot-shaper-indic.cc
- https://github.com/google/fonts/tree/main/ofl/notosansdevanagari
