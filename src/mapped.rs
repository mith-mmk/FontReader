//! Source-preserving shaping results. Byte ranges refer to the original UTF-8
//! input, never normalized text or glyph indices. This is not paragraph layout.
use crate::{GlyphRun, TextDirection};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// Source ownership of one output glyph. Several glyphs may share a range;
/// a ligature may own several graphemes. Ranges are conservative contiguous
/// envelopes, not a claim that every scalar in the range produced this glyph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphSource {
    range: Range<usize>,
    face_index: usize,
    run_index: usize,
}
impl GlyphSource {
    pub(crate) fn new(range: Range<usize>, face_index: usize, run_index: usize) -> Self {
        Self {
            range,
            face_index,
            run_index,
        }
    }
    /// Original input UTF-8 byte range.
    pub fn byte_range(&self) -> Range<usize> {
        self.range.clone()
    }
    /// Selected face index within the family/composite, or zero for one face.
    pub fn face_index(&self) -> usize {
        self.face_index
    }
    /// Shaping segment index, local to this result (not a persistent font ID).
    pub fn run_index(&self) -> usize {
        self.run_index
    }
}

/// An immutable glyph run and its original text ownership. Existing `GlyphRun`
/// and `PositionedGlyph` remain unchanged for source compatibility.
#[derive(Debug, Clone)]
pub struct ShapedText {
    text: String,
    run: GlyphRun,
    sources: Vec<GlyphSource>,
    graphemes: Vec<Range<usize>>,
    direction: TextDirection,
    ligature_carets: Vec<Option<Vec<f32>>>,
    #[cfg(feature = "svg-fonts")]
    svg_placements: crate::commands::SvgPlacements,
}
impl ShapedText {
    pub(crate) fn new(
        text: &str,
        run: GlyphRun,
        sources: Vec<GlyphSource>,
        direction: TextDirection,
    ) -> Self {
        assert_eq!(run.glyphs.len(), sources.len());
        debug_assert!(sources.iter().all(|s| s.range.start <= s.range.end
            && text.is_char_boundary(s.range.start)
            && text.is_char_boundary(s.range.end)));
        let ligature_carets = vec![None; run.glyphs.len()];
        Self {
            ligature_carets,
            #[cfg(feature = "svg-fonts")]
            svg_placements: Default::default(),
            text: text.to_owned(),
            run,
            sources,
            graphemes: text
                .grapheme_indices(true)
                .map(|(i, g)| i..i + g.len())
                .collect(),
            direction,
        }
    }
    pub(crate) fn with_ligature_carets(mut self, carets: Vec<Option<Vec<f32>>>) -> Self {
        assert_eq!(carets.len(), self.run.glyphs.len());
        self.ligature_carets = carets;
        self
    }
    #[cfg(feature = "svg-fonts")]
    pub(crate) fn with_svg_placements(
        mut self,
        svg_placements: crate::commands::SvgPlacements,
    ) -> Self {
        self.svg_placements = svg_placements;
        self
    }

    /// SVG viewport transform in this run's coordinates, using the same
    /// contract as [`crate::SvgGlyphRun::svg_layer_transform`].
    #[cfg(feature = "svg-fonts")]
    pub fn svg_layer_transform(&self, glyph: usize, layer: usize) -> Option<[f32; 6]> {
        self.svg_placements.get(glyph, layer)
    }

    /// Supported font-authored caret candidates, parallel to output glyphs.
    /// Values are logical inline distances from the glyph's initial pen in
    /// output units, not raw font design units. `None` means absent or unsupported
    /// data. Layout validates component count, ownership and monotonic bounds
    /// before preferring these candidates to its explicit approximation.
    pub fn glyph_ligature_carets(&self) -> &[Option<Vec<f32>>] {
        &self.ligature_carets
    }

    /// Unmodified original text, including controls which emit no glyph.
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn glyph_run(&self) -> &GlyphRun {
        &self.run
    }
    /// One entry per glyph, in exactly `glyph_run().glyphs` order.
    pub fn glyph_sources(&self) -> &[GlyphSource] {
        &self.sources
    }
    /// Extended grapheme ranges, including controls and zero-glyph clusters.
    /// Their boundaries are possible logical editing boundaries, not caret geometry.
    pub fn grapheme_ranges(&self) -> &[Range<usize>] {
        &self.graphemes
    }
    pub fn direction(&self) -> TextDirection {
        self.direction
    }
    pub fn into_glyph_run(self) -> GlyphRun {
        self.run
    }
}
