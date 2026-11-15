# Changelog

## 0.0.18 - pending publication

### Added

- Explicit single-line horizontal LTR composite fonts with ordered Unicode-range rules, whole-EGC face selection, and rule-local fallback.
- Per-rule positive finite size multipliers and finite signed baseline shifts, with defaults of `1.0` and `0.0`. Size multiplies the requested nominal size before normal face shaping, preserving line-metric normalization. Positive baseline shifts move glyphs upward in output units without scaling the shift or changing advances.
- Adjacent clusters share shaping context only for the same borrowed face object and exactly equal adjustments, including across rule boundaries. Invalid or overflowing effective sizes are rejected; runs using non-default adjustments also reject nonfinite transformed geometry. Existing face/family APIs are unchanged; vertical, multiline and RTL composition remain future work.

### Fixed

- Reject malformed GSUB/GPOS extension headers before following their offsets, preventing recursive extension parsing. Check extension offset arithmetic and preserve valid 32-bit extension targets (see [validation scope](doc/extension-parser-validation.md)); existing public APIs and dependency requirements are unchanged.
- Filter the current input of ordinary GPOS mark attachment lookups using GDEF, including ExtensionPos. Preserve attachment-target search; the final unanchored-origin policy is described below. This is bounded HarfBuzz-compatible filtering, not general mark-layout parity. See [scope and tests](doc/mark-input-filtering.md).

- Keep SVG path clips and gradients in the same translated coordinate system as their positioned glyph, including composite baseline shifts and preceding-span advances. Plain paths retain their existing export format.
- Bound residual kana mark advance suppression to untouched exact-pair EGCs in GPOS faces without GDEF glyph classes. Preserve effective SinglePos/PairPos ownership, nominal positions, raw text exports and no-layout controls. This removes an extra cell, not mark attachment or ink overlap; see [scope](doc/kana-residual-marks.md).

- Keep the nominal base when a variation sequence is unsupported, including raw UVS glyph/SVG rendering wrappers. Prefer exact cmap14 support only after whole-cluster renderability checks; preserve explicit glyph-zero contextual repair, first-selector handling and marked/RTL continuity. Reject missing per-glyph coverage and out-of-range CFF/CFF2 CharStrings during family probing. Add original MIT fixtures; existing public signatures are unchanged. Font-authored unregistered sequences can differ from browser policy.

- Preserve valid bounded kana/modern Hangul composition beside untouched pre-existing missing glyphs. Keep conservative full-run rollback for unusable substituted outputs, including context-only dependencies, ligatures, expansions and identity substitutions. No public provenance API or general normalization change.
- Apply font-authored GPOS `vert` SinglePos adjustments in vertical runs, including unclassified kana marks. Preserve horizontal/sideways behavior, lookup deduplication, GDEF filtering and static-value limits; this does not add vertical PairPos or generic mark fallback, or change public APIs.

- With `layout`, compose 58 exact Unicode 17 canonical kana pairs before cmap when renderable, preserving source spans and existing vertical GSUB; share bounded invalid-GSUB rollback with modern Hangul. Ainu named sequences and raw/no-layout APIs remain unchanged. Add reproducible MIT fixtures, mapping/exclusion proofs and CFF/CFF2 boundary regressions. Existing public APIs and dependency requirements are unchanged.

- Compose complete modern Hangul LV/LVT/LV+T graphemes before cmap when the face has a renderable syllable. Share family probing/rendering, retain original source characters and distinguish source counts from GSUB components. Restore usable original jamo if downstream GSUB invalidates a new composition, without losing composed-only neighbors. Old Hangul, marked/repeated/compatibility jamo, raw APIs and no-layout behavior remain outside the change.
- Add reproducible original MIT Hangul fixtures, exhaustive 11,172-syllable algorithm checks, direct/family/orientation regressions and CFF/CFF2 missing-target guards. Existing public APIs and dependency requirements are unchanged.

- Add font-aware, transactional Unicode 17 canonical Tibetan decomposition and bounded ordering for fully collapsed consonant/subjoined forms and UU / AA+U / U+AA. Preserve original source/grapheme ownership independently of normalized glyph components and retain originals when coverage or stack validation fails.
- Pin official normalization data with offline hash/generator checks and independent Rust tests. Add MIT fixtures for canonical equivalence, selectors, rollback, missing components, original-grapheme family selection and orientation. II/vocalic R/L/reversed II, compatibility forms and mixed/punctuation runs remain outside this normalization plan. Bounded upper-stack handling is described below. Existing public APIs and dependency requirements are unchanged.

- Add a transactional, whole-run Tibetan `tibt` GSUB plan for ordered nondecomposable single-glyph stacks and simple-vowel alternates. Preserve whole-run presentation context, explicit language selection, and legacy output for mixed/unsupported/incomplete runs. Use exact no-excluded-glyph GDEF proofs, with the bounded upper-stack filtered-context exception described below; general mark skipping remains unsupported; canonical preprocessing is bounded as described above.
- Keep family fallback safe when an isolated Tibetan support probe and a later grouped run choose different shaping paths, by requiring coverage in both. Add reproducible MIT fixtures for stages, ownership, filtering, rollback, context, required features, orientation and missing-glyph fallback. This substitution path does not add GPOS or change existing public APIs or dependency requirements.

- Position adjacent, unligated Tibetan consonant/simple-vowel pairs using `tibt` `abvm`/`blwm` MarkToBase anchors only after existing mark attachment fails. Preserve locale selection and GDEF filtering; this anchor fallback itself does not cover ligated stacks or normalization.

- Select RTL Arabic-style joining forms from pinned Unicode Joining_Type values, rather than font feature availability or immediate glyph neighbors. Transparent marks, tatweel and join controls keep their source-character semantics through ccmp expansion/contraction. Add synthetic public-API regressions; this does not add AAT, full bidi, Syriac joining groups, or general mark-skipping GSUB.

- Apply LTR contextual GSUB and sequence-aware vertical/mixed/sideways substitutions in font-authored LookupList order, preserving source and ligature component metadata through size changes. Keep one orientation feature per route and unchanged layout-disabled behavior.
- Correct context/chaining format 1 record parsing, matched no-op rules, reverse Extension traversal, and bounded nested execution. GDEF-proven mark-free runs retain flagged ligatures; general skip-over-mark GSUB remains unsupported.
- Add small reproducible MIT fixtures for contextual features, IVS cleanup order, nested expansion, required ligatures, orientation selection, and mark anchors. Raw scalar APIs and dependency requirements are unchanged. See the EN/JA feature status for shaping and filtering limits.

### Parser and mark-layout fixes

- Bound SFNT GSUB/GPOS readers to declared table lengths, rejecting offsets that escape a layout table rather than reading adjacent font data.
- Use actual GPOS anchors for mark attachment origins. With GPOS present, unanchored marks retain cursor/authored placement instead of a synthetic GDEF origin; preserve the legacy no-GPOS fallback.
- Select bounded MarkToMark predecessors separately for each lookup with its own GDEF filtering. Preserve original single-source mark ownership through SingleSubst, and keep expansion/contraction and uncertain provenance outside the bounded path. See [scope and tests](doc/mkmk-predecessor-search.md).
- Add internal, opt-in filtered GSUB contexts for the bounded Tibetan plan, preserving physical glyph slots, source ownership and transactional rollback. This is not general skip-over-mark GSUB support.
- Support a bounded Tibetan stack plus one of six upper vowels only with exact Base+Mark ownership and a font-authored `abvm` Base anchor. Preserve ordinary mark priority, locale, filtering and geometry; reject unsupported normalization, uncertain ownership, missing anchors and mixed-script/unsupported runs transactionally; admitted old and new Tibetan neighbors may share the same transaction. See [verification](doc/testing.md#bounded-tibetan-upper-stack-proof-0018).

### Compatibility and verification

- Continue the existing pre-1.0 version cadence at 0.0.18. Composite-font types and methods are additive; existing public signatures, features, dependency requirements, `MIT AND Unicode-3.0` licensing and Rust 1.91 minimum are retained. This is not an API-stability guarantee. Corrected shaping, fallback, metrics and SVG geometry can change output; malformed layout tables can now be rejected earlier.
- Cargo's `fontcore = "0.0.17"` requirement does not select 0.0.18. Update the requirement explicitly after publication; this version-preparation change does not itself publish a crate or release.
- Keep normal tests self-contained with original MIT synthetic fixtures. Unicode normalization, joining and grapheme data remain pinned to 17.0.0, separately from 18.0.0 scalar orientation/enclosing-mark data. Offline generator/hash checks, feature-matrix tests and the Rust 1.91 CI job cover these boundaries; see [testing](doc/testing.md) and [Unicode provenance](data/unicode/README.md).
- Composite fonts remain explicit, single-line horizontal LTR only. Bounded script fixes do not establish full Tibetan/Arabic/Korean shaping, BiDi, AAT or broad HarfBuzz/browser parity. See the [feature status](doc/feature-status.md) for exact exclusions.

## 0.0.17 - 2026-11-14

### Added

- Explicit LTR tate-chu-yoko cell APIs: `shape_tate_chu_yoko_ltr`, `measure_tate_chu_yoko_ltr`, `measure_layout_tate_chu_yoko_ltr`, and `render_svg_tate_chu_yoko_ltr`. They fit a single horizontal run into one upright, scaled-em cell without enlarging it or changing the engine's shaping policy.
- Run all-feature library, integration, and documentation tests in CI on stable Rust and the Rust 1.91 minimum, including feature-gated CFF2, SVG-font, and raw API regressions.

### Fixed

- Apply GPOS PairPos formats 1/2 and Extension lookups with per-lookup GDEF filtering and neighbor selection. Preserve first-match zero records and class-0 matches, accumulate both ValueRecords across distinct lookups without intermediate i16 saturation, and advance the scan cursor according to ValueFormat2 presence. Existing SinglePos and attached-mark advance adjustments are retained.
- Apply horizontal HVAR and gvar metric deltas once, using HVAR advance/LSB entries when available, retaining gvar bearings for a missing optional LSB map, and respecting authoritative mapped-zero values. Cover implicit and explicit advance mappings, including nonidentity maps.
- Apply composite-glyph gvar tuples to component indices and phantom points rather than flattened outline points. Preserve sparse component semantics, selected-face metrics, child-only variations, per-field HVAR/VVAR precedence, and fractional nested XY offsets until the final integer outline conversion. Do not treat `ROUND_XY_TO_GRID` as design-unit rounding in the unhinted path.

### Compatibility

- Preserve existing public signatures, public structs/enums, feature flags, dependencies, licensing, and the Rust 1.91 minimum; the four tate-chu-yoko methods are additive. Corrected positioning, outlines, advances, and bearings can change rendered output, measurement, and SVG bounds.
- This is a version boundary before multi-font composition APIs. Composite glyphs within one face are not composite fonts or multi-face fallback.
- Update dependency requirements explicitly: Cargo's `fontcore = "0.0.16"` requirement does not select 0.0.17.

### Verification and limits

- Add self-contained MIT synthetic fixtures and integration regressions for tate-chu-yoko, PairPos filtering/cursor/ValueRecord semantics, HVAR/gvar metric selection, and composite gvar. The committed composite-gvar oracle was generated independently with fontTools 4.68.0. Normal tests need neither fontTools nor installed fonts; verification instructions and fixture checksums are in `doc/testing.md` and `tests/fixtures/manifest.toml`.
- Tate-chu-yoko is explicit, single-face, single-line, and LTR only. It supports current-color, clip-free filled paths and fits advance width without clipping overhanging ink. Automatic digit grouping, BiDi, multi-font fallback, and multiline composition are not implemented.
- Point-attached composites retain the legacy unvaried-outline fallback; correct point matching is not implemented. `USE_MY_METRICS` inherited phantom positions/hinting are absent: its own phantom deltas are ignored while supported XY outlines vary, with `hmtx`/HVAR and `vmtx`/VVAR metric fallback. Simple-glyph rounding and the public integer outline representation are unchanged; arbitrary rotated/sheared composites are not validated.
- GPOS mark attachment keeps its existing separate filtering behavior. Device/VariationIndex adjustments and broad real-font/HarfBuzz parity remain unimplemented or unverified, respectively. See `doc/feature-status.md` for the detailed support boundaries.

## 0.0.16 - 2026-11-11

### Compatibility

- Update the published `bin-rs` dependency to 0.1.0. Callers passing `bin-rs` readers to low-level APIs must also use `bin-rs` 0.1.0; reader types from `bin-rs` 0.0.11, used by fontcore 0.0.15, are not interchangeable.
- Preserve the high-level APIs, legacy raw API names, default and optional features, and Rust 1.91 minimum. This release changes dependency metadata only; font parsing and shaping code is unchanged.

## 0.0.15 - 2026-11-10

### Compatibility and fixes

- Update the published `bin-rs` dependency to 0.0.11. Callers passing `bin-rs` readers to low-level APIs must also use `bin-rs` 0.0.11; reader types from `bin-rs` 0.0.9, used by published fontcore 0.0.14, are not interchangeable.
- Preserve name-table UTF-16 byte-length handling with the updated reader API, including rejection of odd byte lengths.
- Read the fixed-size OS/2 PANOSE and vendor-ID fields with the updated reader API’s exact-read method.
- Add a UTF-16 name-table regression test. The Rust 1.91 minimum and public feature set are unchanged.

## 0.0.14 - 2026-11-10

### Compatibility and licensing

- Require Rust 1.91 or later, aligned with WML2. CI verifies the minimum version on native and WebAssembly targets.
- Include pinned Unicode data under Unicode-3.0. The package license expression is now `MIT AND Unicode-3.0`; the Rust implementation remains MIT licensed.
- Preserve existing raw API enum names. Header and glyph string formatting now uses `Display` and the standard `ToString` implementation.

### Added

- GPOS SinglePos adjustments and explicit sideways-LTR and mixed vertical-LTR shaping, measurement, and SVG APIs.
- A public vertical-orientation query using pinned Unicode 18.0.0 data.
- Extended grapheme boundaries using pinned Unicode 17.0.0 data and unicode-segmentation 1.13.3, with complete upstream boundary test data.

### Fixed

- Vertical flow without font metrics, vertical origins for TrueType, CFF, CFF2, and static COLR v0, and application of vertical variation metrics.
- Legacy vertical GSUB feature selection, SingleSubst format 1 glyph deltas, lookup ordering, and chaining.
- GPOS mark and pair Y-coordinate handling, plus ZWJ/control and grapheme-aware layout behavior.
- Name-table language-tag offsets and UTF-16 decoding, including rejection of odd byte lengths.

### Verification

- Make normal tests self-contained with committed synthetic font fixtures; keep external font-corpus tests opt-in.
- Add regression coverage for the changes above and preserve the documented limitations in `doc/feature-status.md` and `doc/implementation-status.md`.
