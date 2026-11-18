//! Source-preserving mixed vertical geometry. The existing explicit LTR policy
//! is retained; this is not paragraph BiDi or mixed-direction sideways shaping.
use crate::{
    CaretAffinity, CaretLayout, CaretPosition, CaretQuality, GlyphRun, GlyphSource, ShapedText,
    TextPosition,
};
use std::io::{Error, ErrorKind};
use std::ops::Range;

/// An orientation group before transformation, plus the exact render transform.
/// Local glyph/source coordinates belong to `shaped()`. Rotate clockwise when
/// `is_sideways()`, then translate. Original source offsets add `byte_range().start`.
#[derive(Debug, Clone)]
pub struct VerticalGroup {
    range: Range<usize>,
    shaped: ShapedText,
    sideways: bool,
    translation: (f32, f32),
}
impl VerticalGroup {
    pub(crate) fn new(
        range: Range<usize>,
        shaped: ShapedText,
        sideways: bool,
        translation: (f32, f32),
    ) -> Self {
        Self {
            range,
            shaped,
            sideways,
            translation,
        }
    }
    pub fn byte_range(&self) -> Range<usize> {
        self.range.clone()
    }
    pub fn shaped(&self) -> &ShapedText {
        &self.shaped
    }
    pub fn is_sideways(&self) -> bool {
        self.sideways
    }
    pub fn translation(&self) -> (f32, f32) {
        self.translation
    }
    /// Applies the same y-down clockwise rotation and translation as rendering.
    pub fn transform_point(&self, x: f32, y: f32) -> (f32, f32) {
        let (x, y) = if self.sideways { (-y, x) } else { (x, y) };
        (x + self.translation.0, y + self.translation.1)
    }
}

/// Caret point after the group's render transform, in original UTF-8 coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VerticalCaretPosition {
    position: TextPosition,
    x: f32,
    y: f32,
    quality: CaretQuality,
    source: crate::CaretSource,
    group_index: usize,
}
impl VerticalCaretPosition {
    pub fn text_position(self) -> TextPosition {
        self.position
    }
    pub fn x(self) -> f32 {
        self.x
    }
    pub fn y(self) -> f32 {
        self.y
    }
    pub fn quality(self) -> CaretQuality {
        self.quality
    }
    /// Detailed origin of the caret position; unlike legacy quality, distinguishes GDEF.
    pub fn source(self) -> crate::CaretSource {
        self.source
    }
    pub fn group_index(self) -> usize {
        self.group_index
    }
}

/// Mixed upright/sideways glyphs with explicit per-group source transforms.
/// Drawing succeeds independently of optional caret support. Caret methods
/// inherit the restrictions of `CaretLayout` for every affected local group.
#[derive(Debug, Clone)]
pub struct VerticalMappedLayout {
    text: String,
    run: GlyphRun,
    sources: Vec<GlyphSource>,
    groups: Vec<VerticalGroup>,
    #[cfg(feature = "svg-fonts")]
    svg_placements: crate::commands::SvgPlacements,
}
impl VerticalMappedLayout {
    pub(crate) fn new(
        text: &str,
        run: GlyphRun,
        sources: Vec<GlyphSource>,
        groups: Vec<VerticalGroup>,
    ) -> Self {
        Self {
            text: text.to_owned(),
            run,
            sources,
            groups,
            #[cfg(feature = "svg-fonts")]
            svg_placements: Default::default(),
        }
    }

    #[cfg(feature = "svg-fonts")]
    pub(crate) fn with_svg_placements(
        mut self,
        placements: crate::commands::SvgPlacements,
    ) -> Self {
        self.svg_placements = placements;
        self
    }

    #[cfg(feature = "svg-fonts")]
    pub(crate) fn into_svg_run(self) -> crate::SvgGlyphRun {
        crate::SvgGlyphRun {
            run: self.run,
            placements: self.svg_placements,
        }
    }

    /// SVG viewport transform in the final run's coordinates. The orientation
    /// group rotation and offset are already applied to this matrix.
    #[cfg(feature = "svg-fonts")]
    pub fn svg_layer_transform(&self, glyph: usize, layer: usize) -> Option<[f32; 6]> {
        self.svg_placements.get(glyph, layer)
    }

    pub fn text(&self) -> &str {
        &self.text
    }
    /// Final transformed geometry, identical to the legacy mixed-vertical run.
    pub fn glyph_run(&self) -> &GlyphRun {
        &self.run
    }
    /// Original full-input UTF-8 ownership, parallel to the final glyphs.
    pub fn glyph_sources(&self) -> &[GlyphSource] {
        &self.sources
    }
    pub fn groups(&self) -> &[VerticalGroup] {
        &self.groups
    }

    /// Resolves an EGC boundary. At an orientation boundary, Upstream chooses
    /// the preceding group and Downstream the following group. Their cross-axis
    /// coordinates can differ because sideways centering is part of the transform.
    /// No caret bar height or selection rectangle is inferred.
    pub fn caret_positions(
        &self,
        position: TextPosition,
    ) -> Result<Vec<VerticalCaretPosition>, Error> {
        let offset = position.byte_offset();
        let matches = |g: &VerticalGroup| offset >= g.range.start && offset <= g.range.end;
        let index = if position.affinity() == CaretAffinity::Upstream {
            self.groups.iter().position(matches)
        } else {
            self.groups.iter().rposition(matches)
        }
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "not a source boundary"))?;
        let group = &self.groups[index];
        let layout = CaretLayout::new(&group.shaped)?;
        layout
            .caret_positions(TextPosition::new(
                offset - group.range.start,
                position.affinity(),
            ))?
            .into_iter()
            .map(|caret| self.transform_caret(index, caret))
            .collect()
    }

    /// Chooses the nearest vertical inline (y) stop, then the nearest cross-axis
    /// (x) stop among equally near inline candidates. This inline-first editing policy is not
    /// nearest-ink or Euclidean hit testing. Outside points clamp. Exact ties choose
    /// lower source offset, then the preceding group; the affinity identifies
    /// that side at an orientation boundary. Non-finite coordinates are invalid.
    pub fn hit_test(&self, x: f32, y: f32) -> Result<VerticalCaretPosition, Error> {
        if !x.is_finite() || !y.is_finite() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "hit-test coordinates must be finite",
            ));
        }
        let mut stops = Vec::new();
        for (index, group) in self.groups.iter().enumerate() {
            let layout = CaretLayout::new(&group.shaped)?;
            for &local in layout.stops() {
                let affinity = if local.text_position().byte_offset() == group.shaped.text().len()
                    && index + 1 < self.groups.len()
                {
                    CaretAffinity::Upstream
                } else {
                    CaretAffinity::Downstream
                };
                let mut caret = self.transform_caret(index, local)?;
                caret.position = TextPosition::new(caret.position.byte_offset(), affinity);
                stops.push(caret);
            }
        }
        let first = *stops
            .first()
            .ok_or_else(|| Error::new(ErrorKind::Unsupported, "no caret stops"))?;
        if stops.windows(2).any(|pair| pair[0].y > pair[1].y) {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "nonmonotone mixed vertical carets",
            ));
        }
        let y = (y as f64).clamp(first.y as f64, stops.last().unwrap().y as f64);
        let nearest = stops
            .iter()
            .map(|s| (s.y as f64 - y).abs())
            .fold(f64::INFINITY, f64::min);
        let candidates: Vec<_> = stops
            .into_iter()
            .filter(|s| (s.y as f64 - y).abs() == nearest)
            .collect();
        let min_x = candidates
            .iter()
            .map(|s| s.x as f64)
            .fold(f64::INFINITY, f64::min);
        let max_x = candidates
            .iter()
            .map(|s| s.x as f64)
            .fold(f64::NEG_INFINITY, f64::max);
        let x = (x as f64).clamp(min_x, max_x);
        let mut best = candidates[0];
        let mut distance = (best.x as f64 - x).abs();
        for candidate in candidates.into_iter().skip(1) {
            let delta = (candidate.x as f64 - x).abs();
            if delta < distance {
                best = candidate;
                distance = delta;
            }
        }
        Ok(best)
    }
    fn transform_caret(
        &self,
        index: usize,
        caret: CaretPosition,
    ) -> Result<VerticalCaretPosition, Error> {
        let group = &self.groups[index];
        let (x, y) = group.transform_point(caret.x(), caret.y());
        if !x.is_finite() || !y.is_finite() {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "non-finite transformed caret",
            ));
        }
        Ok(VerticalCaretPosition {
            position: TextPosition::new(
                group.range.start + caret.text_position().byte_offset(),
                caret.text_position().affinity(),
            ),
            x,
            y,
            quality: caret.quality(),
            source: caret.source(),
            group_index: index,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FontFile;
    const FONT: &[u8] = include_bytes!("../tests/fixtures/mixed-vertical-ltr.ttc");
    fn position(byte: usize, affinity: CaretAffinity) -> TextPosition {
        TextPosition::new(byte, affinity)
    }
    #[test]
    fn geometry_and_original_ownership_match_legacy() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(0).unwrap();
        let engine = face.engine().with_font_size(1000.0);
        for text in [
            "",
            "AB",
            "AあB",
            "fiあfi",
            "A\u{308}あ",
            "A\u{20dd}B",
            "あ\u{fe0f}A",
        ] {
            let mapped = engine.shape_vertical_mixed_ltr_mapped(text).unwrap();
            assert_eq!(mapped.text(), text);
            assert_eq!(
                format!("{:?}", mapped.glyph_run()),
                format!("{:?}", engine.shape_vertical_mixed_ltr(text).unwrap()),
                "{text:?}"
            );
            assert_eq!(
                mapped.glyph_run().glyphs.len(),
                mapped.glyph_sources().len()
            );
            for source in mapped.glyph_sources() {
                assert!(text.get(source.byte_range()).is_some());
                assert_eq!(source.face_index(), 0);
                let group = &mapped.groups()[source.run_index()];
                assert!(source.byte_range().start >= group.byte_range().start);
                assert!(source.byte_range().end <= group.byte_range().end);
            }
        }
    }
    #[test]
    fn feature_variants_keep_exact_geometry_and_rejection_policy() {
        let file = FontFile::from_buffer(FONT).unwrap();
        for index in 0..file.face_count() {
            let face = file.face(index).unwrap();
            let engine = face.engine().with_font_size(32.0);
            for text in ["AあB", "fiあfi", "A\u{308}あ", "A\u{20dd}B"] {
                match (
                    engine.shape_vertical_mixed_ltr(text),
                    engine.shape_vertical_mixed_ltr_mapped(text),
                ) {
                    (Ok(legacy), Ok(mapped)) => assert_eq!(
                        format!("{legacy:?}"),
                        format!("{:?}", mapped.glyph_run()),
                        "face {index} {text:?}"
                    ),
                    (Err(a), Err(b)) => assert_eq!(a.kind(), b.kind()),
                    (a, b) => panic!("face {index} {text:?}: legacy={a:?} mapped={b:?}"),
                }
            }
        }
    }

    #[test]
    fn transforms_and_affinity_preserve_orientation_boundary() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(0).unwrap();
        let engine = face.engine().with_font_size(1000.0);
        let mapped = engine.shape_vertical_mixed_ltr_mapped("AあB").unwrap();
        assert_eq!(mapped.groups().len(), 3);
        assert!(mapped.groups()[0].is_sideways());
        assert!(!mapped.groups()[1].is_sideways());
        let a = mapped
            .caret_positions(position(1, CaretAffinity::Upstream))
            .unwrap()[0];
        let b = mapped
            .caret_positions(position(1, CaretAffinity::Downstream))
            .unwrap()[0];
        assert_eq!(a.y(), b.y());
        assert_ne!(a.x(), b.x());
        assert_eq!(a.group_index(), 0);
        assert_eq!(b.group_index(), 1);
        assert_eq!(mapped.hit_test(a.x(), a.y()).unwrap().group_index(), 0);
        assert_eq!(mapped.hit_test(b.x(), b.y()).unwrap().group_index(), 1);
        for (i, group) in mapped.groups().iter().enumerate() {
            let local = CaretLayout::new(group.shaped()).unwrap();
            let end = local
                .caret_positions(position(
                    group.shaped().text().len(),
                    CaretAffinity::Upstream,
                ))
                .unwrap()[0];
            let expected = group.transform_point(end.x(), end.y());
            let actual = mapped
                .caret_positions(position(group.byte_range().end, CaretAffinity::Upstream))
                .unwrap()[0];
            assert_eq!((actual.x(), actual.y()), expected);
            assert_eq!(actual.group_index(), i);
        }
        assert_eq!(
            mapped
                .hit_test(0.0, f32::MAX)
                .unwrap()
                .text_position()
                .byte_offset(),
            5
        );
        assert_eq!(
            mapped
                .hit_test(0.0, -f32::MAX)
                .unwrap()
                .text_position()
                .byte_offset(),
            0
        );
        assert!(mapped.hit_test(f32::NAN, 0.0).is_err());
        assert!(mapped
            .caret_positions(position(2, CaretAffinity::Downstream))
            .is_err());
    }
    #[test]
    fn mapped_transforms_reject_overflow() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(0).unwrap();
        let engine = face.engine().with_font_size(f32::MAX);
        for text in ["AAAAAAAA", "ああああ", "AあAあAあAあ"] {
            assert!(
                engine.shape_vertical_mixed_ltr_mapped(text).is_err(),
                "{text:?}"
            );
        }
    }
    #[test]
    fn empty_combining_and_control_limits() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(0).unwrap();
        let engine = face.engine();
        let empty = engine.shape_vertical_mixed_ltr_mapped("").unwrap();
        assert_eq!(
            empty
                .hit_test(50.0, 50.0)
                .unwrap()
                .text_position()
                .byte_offset(),
            0
        );
        let combining = engine
            .shape_vertical_mixed_ltr_mapped("A\u{308}あ")
            .unwrap();
        assert!(combining
            .caret_positions(position(1, CaretAffinity::Downstream))
            .is_err());
        assert!(combining
            .caret_positions(position(3, CaretAffinity::Downstream))
            .is_ok());
        for text in ["A\nB", "A\tB", "A\rB"] {
            assert!(engine.shape_vertical_mixed_ltr_mapped(text).is_err());
        }
        let orphan = engine.shape_vertical_mixed_ltr_mapped("\u{fe0f}A").unwrap();
        assert!(orphan.hit_test(0.0, 0.0).is_err());
    }
}
