//! Bounded advance-based caret geometry for one explicitly directed line.
//!
//! This module does not perform paragraph BiDi, line breaking, mixed vertical
//! orientation, or raw GDEF parsing. Supported font caret candidates are consumed
//! from mapped shaping metadata. Coordinates are relative to the
//! run's initial pen `(0, 0)`, not ink bounds or positioned mark origins.
//! Composite baseline shifts are deliberately ignored: returned points remain
//! on the common initial pen baseline, not the shifted glyph baseline. No caret
//! height, line box or selection rectangle is implied by these points.
use crate::{ShapedText, TextDirection};
use std::io::{Error, ErrorKind};

/// Which logical side of a boundary is intended. In this single-line model
/// both affinities have the same geometry; paragraph layout may distinguish them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaretAffinity {
    Upstream,
    Downstream,
}

/// An original UTF-8 byte boundary and its logical affinity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPosition {
    byte_offset: usize,
    affinity: CaretAffinity,
}
impl TextPosition {
    pub fn new(byte_offset: usize, affinity: CaretAffinity) -> Self {
        Self {
            byte_offset,
            affinity,
        }
    }
    pub fn byte_offset(self) -> usize {
        self.byte_offset
    }
    pub fn affinity(self) -> CaretAffinity {
        self.affinity
    }
}

/// Legacy conservative classification: group boundaries versus internal stops.
/// Internal stops retain `Approximate` for compatibility even when GDEF provides
/// their position. Use [`CaretPosition::source`] to distinguish authored carets
/// from interpolation; this enum intentionally gains no new variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaretQuality {
    AdvanceBoundary,
    Approximate,
}

/// Provenance of a caret coordinate, independent of the legacy quality flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaretSource {
    /// A shaped group/advance boundary.
    Advance,
    /// A validated font-authored GDEF ligature caret.
    FontAuthored,
    /// Equal subdivision because authored caret data could not be used.
    Approximate,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretPosition {
    position: TextPosition,
    x: f32,
    y: f32,
    quality: CaretQuality,
    source: CaretSource,
}
impl CaretPosition {
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
    /// Authoritative provenance; `quality()` remains a conservative legacy flag.
    pub fn source(self) -> CaretSource {
        self.source
    }
}

/// Single-line caret stops at extended grapheme boundaries only.
///
/// Advances must be finite and nonnegative on the inline axis, with zero
/// cross-axis advance. Source ownership must cover the input in monotonically
/// ordered, non-overlapping groups. Multiple glyphs may share a group.
/// Unsupported mappings are rejected rather than guessed. Tabs, line controls,
/// BiDi formatting controls and zero-glyph source gaps are not supported.
#[derive(Debug, Clone)]
pub struct CaretLayout {
    stops: Vec<CaretPosition>,
    vertical: bool,
}
impl CaretLayout {
    pub(crate) fn stops(&self) -> &[CaretPosition] {
        &self.stops
    }

    pub fn new(shaped: &ShapedText) -> Result<Self, Error> {
        let unsupported = |message| Error::new(ErrorKind::Unsupported, message);
        if shaped.text().chars().any(|c| {
            c.is_control()
                || matches!(c,
            '\u{2028}' | '\u{2029}' | '\u{061c}' | '\u{200e}' | '\u{200f}' |
            '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        }) {
            return Err(unsupported(
                "caret layout requires one line without tab or BiDi controls",
            ));
        }
        let vertical = shaped.direction() == TextDirection::TopToBottom;
        let sign = if shaped.direction() == TextDirection::RightToLeft {
            -1.0
        } else {
            1.0
        };
        // Groups are contiguous source envelopes. Overlapping glyph ownership
        // is merged before distributing advances; no stop is added inside an EGC.
        let mut groups: Vec<(std::ops::Range<usize>, f32, Option<usize>)> = Vec::new();
        for (glyph_index, (glyph, source)) in shaped
            .glyph_run()
            .glyphs
            .iter()
            .zip(shaped.glyph_sources())
            .enumerate()
        {
            let m = &glyph.glyph.metrics;
            let (advance, cross) = if vertical {
                (m.advance_y, m.advance_x)
            } else {
                (m.advance_x, m.advance_y)
            };
            if !advance.is_finite() || !cross.is_finite() || advance < 0.0 || cross != 0.0 {
                return Err(unsupported("caret layout requires finite nonnegative inline advances and zero cross advance"));
            }
            let range = source.byte_range();
            if range.is_empty() {
                return Err(unsupported("empty glyph source ownership"));
            }
            if let Some((last, width, owner)) = groups.last_mut() {
                if range.start < last.start {
                    return Err(unsupported(
                        "reordered source ownership needs a visual layout layer",
                    ));
                }
                if range.start < last.end {
                    last.end = last.end.max(range.end);
                    *width += advance;
                    *owner = None; // Multiple glyphs make authored component ownership ambiguous.
                    if !width.is_finite() {
                        return Err(unsupported("caret advance overflow"));
                    }
                    continue;
                }
            }
            groups.push((range, advance, Some(glyph_index)));
        }
        let mut boundaries = vec![0];
        boundaries.extend(shaped.grapheme_ranges().iter().map(|g| g.end));
        let mut stops = vec![CaretPosition {
            position: TextPosition::new(0, CaretAffinity::Downstream),
            x: 0.0,
            y: 0.0,
            quality: CaretQuality::AdvanceBoundary,
            source: CaretSource::Advance,
        }];
        let mut source_cursor = 0;
        let mut pen = 0.0;
        for (range, advance, owner) in groups {
            let start_index = boundaries.binary_search(&range.start);
            let end_index = boundaries.binary_search(&range.end);
            let (Ok(start_index), Ok(end_index)) = (start_index, end_index) else {
                return Err(unsupported(
                    "partial grapheme ownership needs a richer layout layer",
                ));
            };
            if range.start != source_cursor {
                return Err(unsupported("source gaps need a richer layout layer"));
            }
            let inside = &boundaries[start_index + 1..=end_index];
            let count = inside.len();
            let authored = owner.and_then(|index| authored_carets(shaped, index, count, advance));
            for (i, &byte) in inside.iter().enumerate() {
                let (distance, source) = if i + 1 == count {
                    (advance, CaretSource::Advance)
                } else if let Some(authored) = authored {
                    (authored[i], CaretSource::FontAuthored)
                } else {
                    (
                        advance * ((i + 1) as f32 / count as f32),
                        CaretSource::Approximate,
                    )
                };
                let inline = sign * (pen + distance);
                if !inline.is_finite() {
                    return Err(unsupported("caret advance overflow"));
                }
                stops.push(CaretPosition {
                    position: TextPosition::new(byte, CaretAffinity::Downstream),
                    x: if vertical { 0.0 } else { inline },
                    y: if vertical { inline } else { 0.0 },
                    source,
                    quality: if i + 1 == count {
                        CaretQuality::AdvanceBoundary
                    } else {
                        CaretQuality::Approximate
                    },
                });
            }
            pen += advance;
            source_cursor = range.end;
        }
        if source_cursor != shaped.text().len() {
            return Err(unsupported(
                "unowned source text needs a richer layout layer",
            ));
        }
        Ok(Self { stops, vertical })
    }

    /// Returns the one position for a valid EGC boundary. Other byte offsets,
    /// including scalar boundaries inside combining clusters, are invalid.
    pub fn caret_positions(&self, position: TextPosition) -> Result<Vec<CaretPosition>, Error> {
        let mut caret = *self
            .stops
            .iter()
            .find(|s| s.position.byte_offset == position.byte_offset)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    "not an available grapheme boundary",
                )
            })?;
        caret.position = position;
        Ok(vec![caret])
    }

    /// Chooses the nearest inline stop, ignoring the cross-axis coordinate.
    /// Outside points clamp to endpoints. Exact ties (including zero advances)
    /// choose the lower logical byte offset. Both coordinates must be finite.
    /// The returned affinity is Downstream; there is no paragraph-side inference.
    pub fn hit_test(&self, x: f32, y: f32) -> Result<CaretPosition, Error> {
        if !x.is_finite() || !y.is_finite() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "hit-test coordinates must be finite",
            ));
        }
        let value = if self.vertical { y as f64 } else { x as f64 };
        // Clamp before subtracting: an extreme finite input can otherwise round
        // every distance to the same f64 value and incorrectly select byte zero.
        let first = self.stops[0];
        let last = self.stops[self.stops.len() - 1];
        let a = if self.vertical { first.y } else { first.x } as f64;
        let b = if self.vertical { last.y } else { last.x } as f64;
        let value = value.clamp(a.min(b), a.max(b));
        let mut best = self.stops[0];
        let mut distance = f64::INFINITY;
        for &stop in &self.stops {
            let coordinate = if self.vertical {
                stop.y as f64
            } else {
                stop.x as f64
            };
            let delta = (coordinate - value).abs();
            if delta < distance {
                best = stop;
                distance = delta;
            }
        }
        Ok(best)
    }
}

/// A single owning glyph must provide exactly one strictly interior, increasing
/// candidate per internal visible-grapheme boundary. Do not sort, clamp or infer
/// missing components: those cases intentionally use explicit approximation.
pub(crate) fn authored_carets(
    shaped: &ShapedText,
    glyph_index: usize,
    count: usize,
    advance: f32,
) -> Option<&[f32]> {
    if count < 2 || !advance.is_finite() || advance <= 0.0 {
        return None;
    }
    let values = shaped
        .glyph_ligature_carets()
        .get(glyph_index)?
        .as_deref()?;
    if values.len() != count - 1 {
        return None;
    }
    let mut previous = 0.0;
    for &value in values {
        if !value.is_finite() || value <= previous || value >= advance {
            return None;
        }
        previous = value;
    }
    Some(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FontFile, GlyphRun, GlyphSource};
    const FONT: &[u8] = include_bytes!("../tests/fixtures/european-fallback.ttc");
    #[test]
    fn ligature_and_combining_stops() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(1).unwrap();
        let engine = face.engine().with_font_size(1000.0);
        let mapped = engine.shape_mapped("fia\u{308}").unwrap();
        let layout = CaretLayout::new(&mapped).unwrap();
        let internal = layout
            .caret_positions(TextPosition::new(1, CaretAffinity::Upstream))
            .unwrap()[0];
        if cfg!(feature = "layout") {
            assert_eq!(internal.quality(), CaretQuality::Approximate);
        }
        assert_eq!(internal.text_position().affinity(), CaretAffinity::Upstream);
        assert!(layout
            .caret_positions(TextPosition::new(3, CaretAffinity::Downstream))
            .is_err());
        assert_eq!(
            layout
                .hit_test(-100.0, 200.0)
                .unwrap()
                .text_position()
                .byte_offset(),
            0
        );
        assert_eq!(
            layout
                .hit_test(10000.0, -200.0)
                .unwrap()
                .text_position()
                .byte_offset(),
            5
        );
        assert!(layout.hit_test(f32::NAN, 0.0).is_err());
    }
    #[test]
    fn extreme_finite_points_clamp_before_distance_comparison() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(1).unwrap();
        for engine in [
            face.engine(),
            face.engine().with_right_to_left(),
            face.engine().with_vertical_flow(),
        ] {
            let shaped = engine.shape_mapped("BB").unwrap();
            let layout = CaretLayout::new(&shaped).unwrap();
            let (x, y) = match shaped.direction() {
                TextDirection::LeftToRight => (f32::MAX, 0.0),
                TextDirection::RightToLeft => (-f32::MAX, 0.0),
                TextDirection::TopToBottom => (0.0, f32::MAX),
            };
            assert_eq!(
                layout.hit_test(x, y).unwrap().text_position().byte_offset(),
                2
            );
            assert_eq!(
                layout
                    .hit_test(-x, -y)
                    .unwrap()
                    .text_position()
                    .byte_offset(),
                0
            );
        }
    }

    #[test]
    fn empty_and_unsupported_controls() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(1).unwrap();
        let engine = face.engine();
        let empty = CaretLayout::new(&engine.shape_mapped("").unwrap()).unwrap();
        assert_eq!(empty.hit_test(100.0, 0.0).unwrap().x(), 0.0);
        for text in ["B\nB", "B\tB", "B\u{202e}B"] {
            assert_eq!(
                CaretLayout::new(&engine.shape_mapped(text).unwrap())
                    .unwrap_err()
                    .kind(),
                ErrorKind::Unsupported
            );
        }
    }
    #[test]
    fn explicit_axes_and_invalid_advances() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(1).unwrap();
        let original = face.engine().with_font_size(1000.0).shape("B").unwrap();
        for direction in [
            TextDirection::LeftToRight,
            TextDirection::RightToLeft,
            TextDirection::TopToBottom,
        ] {
            let mut run = original.clone();
            run.glyphs[0].glyph.metrics.advance_x = if direction == TextDirection::TopToBottom {
                0.0
            } else {
                800.0
            };
            run.glyphs[0].glyph.metrics.advance_y = if direction == TextDirection::TopToBottom {
                800.0
            } else {
                0.0
            };
            let shaped = ShapedText::new("B", run, vec![GlyphSource::new(0..1, 0, 0)], direction);
            let layout = CaretLayout::new(&shaped).unwrap();
            let end = layout
                .caret_positions(TextPosition::new(1, CaretAffinity::Downstream))
                .unwrap()[0];
            assert_eq!(
                (end.x(), end.y()),
                match direction {
                    TextDirection::LeftToRight => (800.0, 0.0),
                    TextDirection::RightToLeft => (-800.0, 0.0),
                    TextDirection::TopToBottom => (0.0, 800.0),
                }
            );
            assert_eq!(
                layout
                    .hit_test(end.x(), end.y())
                    .unwrap()
                    .text_position()
                    .byte_offset(),
                1
            );
        }
        let mut run = original;
        run.glyphs[0].glyph.metrics.advance_x = -1.0;
        assert!(CaretLayout::new(&ShapedText::new(
            "B",
            run,
            vec![GlyphSource::new(0..1, 0, 0)],
            TextDirection::LeftToRight
        ))
        .is_err());
        assert!(CaretLayout::new(&ShapedText::new(
            "B",
            GlyphRun::default(),
            vec![],
            TextDirection::LeftToRight
        ))
        .is_err());
    }
    #[test]
    fn spaces_zero_advances_and_placement_do_not_use_ink() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(1).unwrap();
        let mut run = face.engine().shape("BB").unwrap();
        run.glyphs[0].x = 9000.0;
        run.glyphs[0].glyph.metrics.advance_x = 0.0;
        run.glyphs[1].glyph.metrics.advance_x = 20.0;
        // Empty layers stand for blank glyphs; ink and mark placement do not
        // determine pen boundaries.
        for glyph in &mut run.glyphs {
            glyph.glyph.layers.clear();
        }
        let shaped = ShapedText::new(
            " B",
            run,
            vec![GlyphSource::new(0..1, 0, 0), GlyphSource::new(1..2, 0, 0)],
            TextDirection::LeftToRight,
        );
        let layout = CaretLayout::new(&shaped).unwrap();
        assert_eq!(
            layout
                .hit_test(0.0, 0.0)
                .unwrap()
                .text_position()
                .byte_offset(),
            0
        );
        assert_eq!(
            layout
                .hit_test(10.0, 0.0)
                .unwrap()
                .text_position()
                .byte_offset(),
            0
        );
        assert_eq!(
            layout
                .hit_test(11.0, 0.0)
                .unwrap()
                .text_position()
                .byte_offset(),
            2
        );
        assert_eq!(
            layout
                .caret_positions(TextPosition::new(1, CaretAffinity::Downstream))
                .unwrap()[0]
                .x(),
            0.0
        );
    }
    #[test]
    fn real_engine_direction_and_endpoint_agree() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(1).unwrap();
        for direction in [
            TextDirection::LeftToRight,
            TextDirection::RightToLeft,
            TextDirection::TopToBottom,
        ] {
            let options = face
                .engine()
                .options()
                .with_text_direction(direction)
                .with_font_size(1000.0);
            let mapped = face
                .engine()
                .with_options(options)
                .shape_mapped("BB")
                .unwrap();
            let layout = CaretLayout::new(&mapped).unwrap();
            let endpoint = layout
                .caret_positions(TextPosition::new(2, CaretAffinity::Downstream))
                .unwrap()[0];
            let x: f32 = mapped
                .glyph_run()
                .glyphs
                .iter()
                .map(|g| g.glyph.metrics.advance_x)
                .sum();
            let y: f32 = mapped
                .glyph_run()
                .glyphs
                .iter()
                .map(|g| g.glyph.metrics.advance_y)
                .sum();
            assert_eq!(
                endpoint.x(),
                if direction == TextDirection::RightToLeft {
                    -x
                } else {
                    x
                }
            );
            assert_eq!(endpoint.y(), y);
            assert_eq!(
                layout
                    .hit_test(endpoint.x(), endpoint.y())
                    .unwrap()
                    .text_position()
                    .byte_offset(),
                2
            );
        }
    }
    #[test]
    fn rejected_source_order_and_overflow_are_explicit() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(1).unwrap();
        let run = face.engine().shape("BB").unwrap();
        let reordered = ShapedText::new(
            "BB",
            run.clone(),
            vec![GlyphSource::new(1..2, 0, 0), GlyphSource::new(0..1, 0, 0)],
            TextDirection::LeftToRight,
        );
        assert_eq!(
            CaretLayout::new(&reordered).unwrap_err().kind(),
            ErrorKind::Unsupported
        );
        // Overlap reaching back before the group start is not silently sorted.
        let overlap = ShapedText::new(
            "BBB",
            run.clone(),
            vec![GlyphSource::new(1..3, 0, 0), GlyphSource::new(0..2, 0, 0)],
            TextDirection::LeftToRight,
        );
        assert!(CaretLayout::new(&overlap).is_err());
        for width in [f32::MAX, f32::INFINITY, f32::NAN] {
            let mut invalid = run.clone();
            for glyph in &mut invalid.glyphs {
                glyph.glyph.metrics.advance_x = width;
            }
            let shaped = ShapedText::new(
                "BB",
                invalid,
                vec![GlyphSource::new(0..1, 0, 0), GlyphSource::new(1..2, 0, 0)],
                TextDirection::LeftToRight,
            );
            assert_eq!(
                CaretLayout::new(&shaped).unwrap_err().kind(),
                ErrorKind::Unsupported
            );
        }
    }
    fn authored_sample(text: &str, candidates: Vec<f32>, direction: TextDirection) -> ShapedText {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(1).unwrap();
        let mut run = face.engine().shape("B").unwrap();
        run.glyphs[0].glyph.metrics.advance_x = if direction == TextDirection::TopToBottom {
            0.0
        } else {
            800.0
        };
        run.glyphs[0].glyph.metrics.advance_y = if direction == TextDirection::TopToBottom {
            800.0
        } else {
            0.0
        };
        ShapedText::new(
            text,
            run,
            vec![GlyphSource::new(0..text.len(), 0, 0)],
            direction,
        )
        .with_ligature_carets(vec![Some(candidates)])
    }
    #[test]
    fn authored_carets_prefer_nonuniform_coordinates_without_changing_legacy_quality() {
        for direction in [
            TextDirection::LeftToRight,
            TextDirection::RightToLeft,
            TextDirection::TopToBottom,
        ] {
            let shaped = authored_sample("ffi", vec![200.0, 600.0], direction);
            let layout = CaretLayout::new(&shaped).unwrap();
            for (offset, distance) in [(1, 200.0), (2, 600.0)] {
                let caret = layout
                    .caret_positions(TextPosition::new(offset, CaretAffinity::Downstream))
                    .unwrap()[0];
                assert_eq!(caret.source(), CaretSource::FontAuthored);
                assert_eq!(caret.quality(), CaretQuality::Approximate);
                assert_eq!(
                    (caret.x(), caret.y()),
                    match direction {
                        TextDirection::LeftToRight => (distance, 0.0),
                        TextDirection::RightToLeft => (-distance, 0.0),
                        TextDirection::TopToBottom => (0.0, distance),
                    }
                );
            }
            assert_eq!(
                layout
                    .caret_positions(TextPosition::new(3, CaretAffinity::Downstream))
                    .unwrap()[0]
                    .source(),
                CaretSource::Advance
            );
        }
    }
    #[test]
    fn invalid_or_ambiguous_authored_candidates_fall_back_explicitly() {
        for values in [
            vec![],
            vec![100.0],
            vec![100.0, 200.0, 300.0],
            vec![0.0, 600.0],
            vec![200.0, 800.0],
            vec![200.0, 200.0],
            vec![600.0, 200.0],
            vec![f32::NAN, 600.0],
            vec![200.0, f32::INFINITY],
            vec![-1.0, 600.0],
        ] {
            let shaped = authored_sample("ffi", values, TextDirection::LeftToRight);
            let layout = CaretLayout::new(&shaped).unwrap();
            let caret = layout
                .caret_positions(TextPosition::new(1, CaretAffinity::Downstream))
                .unwrap()[0];
            assert_eq!(caret.source(), CaretSource::Approximate);
            assert_eq!(caret.x(), 800.0 * (1.0 / 3.0));
        }
        let shaped = authored_sample("fi", vec![200.0], TextDirection::LeftToRight);
        let mut run = shaped.glyph_run().clone();
        run.glyphs.push(run.glyphs[0].clone());
        let ambiguous = ShapedText::new(
            "fi",
            run,
            vec![GlyphSource::new(0..2, 0, 0), GlyphSource::new(0..2, 0, 0)],
            TextDirection::LeftToRight,
        )
        .with_ligature_carets(vec![Some(vec![200.0]), None]);
        let caret = CaretLayout::new(&ambiguous)
            .unwrap()
            .caret_positions(TextPosition::new(1, CaretAffinity::Downstream))
            .unwrap()[0];
        assert_eq!(caret.source(), CaretSource::Approximate);
        assert_eq!(caret.x(), 800.0);
    }
    #[cfg(feature = "layout")]
    #[test]
    fn gdef_fixture_is_preferred_and_unsupported_records_fall_back() {
        let file =
            FontFile::from_buffer(include_bytes!("../tests/fixtures/gdef-carets.ttc")).unwrap();
        let face = file.face(0).unwrap();
        for (direction, expected) in [
            (TextDirection::LeftToRight, 220.0),
            (TextDirection::RightToLeft, -680.0),
            (TextDirection::TopToBottom, 530.0),
        ] {
            let engine = face.engine().with_options(
                face.engine()
                    .options()
                    .with_font_size(1000.0)
                    .with_text_direction(direction),
            );
            let mapped = engine.shape_mapped("fi").unwrap();
            let caret = CaretLayout::new(&mapped)
                .unwrap()
                .caret_positions(TextPosition::new(1, CaretAffinity::Downstream))
                .unwrap()[0];
            assert_eq!(caret.source(), CaretSource::FontAuthored);
            assert_eq!(
                if direction == TextDirection::TopToBottom {
                    caret.y()
                } else {
                    caret.x()
                },
                expected
            );
            assert_eq!(caret.quality(), CaretQuality::Approximate);
        }
        for index in 2..6 {
            let face = file.face(index).unwrap();
            let mapped = face
                .engine()
                .with_font_size(1000.0)
                .shape_mapped("fi")
                .unwrap();
            let caret = CaretLayout::new(&mapped)
                .unwrap()
                .caret_positions(TextPosition::new(1, CaretAffinity::Downstream))
                .unwrap()[0];
            assert_eq!(caret.source(), CaretSource::Approximate, "face {index}");
            assert_eq!(caret.x(), 450.0, "face {index}");
        }
    }
    #[cfg(feature = "layout")]
    #[test]
    fn static_format_two_uses_real_points_and_axis_specific_metric_origins() {
        let file =
            FontFile::from_buffer(include_bytes!("../tests/fixtures/gdef-carets.ttc")).unwrap();
        let face = file.face(1).unwrap();
        for (text, ltr, rtl, ttb) in [("fi", 280.0, -620.0, 750.0), ("fl", 830.0, -270.0, 250.0)] {
            for (direction, expected) in [
                (TextDirection::LeftToRight, ltr),
                (TextDirection::RightToLeft, rtl),
                (TextDirection::TopToBottom, ttb),
            ] {
                let options = face
                    .engine()
                    .options()
                    .with_font_size(1000.0)
                    .with_text_direction(direction);
                let mapped = face
                    .engine()
                    .with_options(options)
                    .shape_mapped(text)
                    .unwrap();
                let caret = CaretLayout::new(&mapped)
                    .unwrap()
                    .caret_positions(TextPosition::new(1, CaretAffinity::Downstream))
                    .unwrap()[0];
                assert_eq!(
                    caret.source(),
                    CaretSource::FontAuthored,
                    "{text} {direction:?}"
                );
                assert_eq!(caret.quality(), CaretQuality::Approximate);
                assert_eq!(
                    if direction == TextDirection::TopToBottom {
                        caret.y()
                    } else {
                        caret.x()
                    },
                    expected,
                    "{text} {direction:?}"
                );
            }
        }
    }
}
