//! Bounded advance-based caret geometry for one explicitly directed line.
//!
//! This module does not perform paragraph BiDi, line breaking, mixed vertical
//! orientation, or GDEF ligature-caret lookup. Coordinates are relative to the
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

/// Whether a position follows a shaped advance or divides a multi-grapheme
/// ownership group. Approximate positions are not font-authored ligature carets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaretQuality {
    AdvanceBoundary,
    Approximate,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretPosition {
    position: TextPosition,
    x: f32,
    y: f32,
    quality: CaretQuality,
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
        let mut groups: Vec<(std::ops::Range<usize>, f32)> = Vec::new();
        for (glyph, source) in shaped.glyph_run().glyphs.iter().zip(shaped.glyph_sources()) {
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
            if let Some((last, width)) = groups.last_mut() {
                if range.start < last.start {
                    return Err(unsupported(
                        "reordered source ownership needs a visual layout layer",
                    ));
                }
                if range.start < last.end {
                    last.end = last.end.max(range.end);
                    *width += advance;
                    if !width.is_finite() {
                        return Err(unsupported("caret advance overflow"));
                    }
                    continue;
                }
            }
            groups.push((range, advance));
        }
        let mut boundaries = vec![0];
        boundaries.extend(shaped.grapheme_ranges().iter().map(|g| g.end));
        let mut stops = vec![CaretPosition {
            position: TextPosition::new(0, CaretAffinity::Downstream),
            x: 0.0,
            y: 0.0,
            quality: CaretQuality::AdvanceBoundary,
        }];
        let mut source_cursor = 0;
        let mut pen = 0.0;
        for (range, advance) in groups {
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
            for (i, &byte) in inside.iter().enumerate() {
                let inline = sign * (pen + advance * ((i + 1) as f32 / count as f32));
                if !inline.is_finite() {
                    return Err(unsupported("caret advance overflow"));
                }
                stops.push(CaretPosition {
                    position: TextPosition::new(byte, CaretAffinity::Downstream),
                    x: if vertical { 0.0 } else { inline },
                    y: if vertical { inline } else { 0.0 },
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
}
