//! Explicitly directed hard-line layout. Only LF and CRLF break lines.
//!
//! This is not automatic wrapping, paragraph BiDi, mixed vertical orientation,
//! or font fallback. Each line is shaped with the same selected engine face.
//! Line origins follow a fixed baseline step, not inferred ink or line boxes.
use crate::{
    CaretLayout, CaretPosition, CaretQuality, CompositeFont, FontEngine, FontFamily, FontOptions,
    ShapedText, TextDirection, TextPosition,
};
use std::io::{Error, ErrorKind};
use std::ops::Range;

/// A shaped content line plus its source range and translation. To render,
/// draw `shaped().glyph_run()` translated by `(origin_x(), origin_y())`.
/// Source ranges inside `shaped()` are line-local; add `content_range().start`.
#[derive(Debug, Clone)]
pub struct HardLine {
    content: Range<usize>,
    line_break: Range<usize>,
    shaped: ShapedText,
    carets: CaretLayout,
    origin_x: f32,
    origin_y: f32,
}
impl HardLine {
    /// Original UTF-8 range excluding the line break.
    pub fn content_range(&self) -> Range<usize> {
        self.content.clone()
    }
    /// LF or CRLF range; empty on the final line.
    pub fn break_range(&self) -> Range<usize> {
        self.line_break.clone()
    }
    pub fn shaped(&self) -> &ShapedText {
        &self.shaped
    }
    pub fn origin_x(&self) -> f32 {
        self.origin_x
    }
    pub fn origin_y(&self) -> f32 {
        self.origin_y
    }
}

/// A caret in layout coordinates, with an original-input UTF-8 position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineCaretPosition {
    position: TextPosition,
    x: f32,
    y: f32,
    quality: CaretQuality,
    source: crate::CaretSource,
    line_index: usize,
}
impl LineCaretPosition {
    pub(crate) fn new(
        position: TextPosition,
        x: f32,
        y: f32,
        quality: CaretQuality,
        source: crate::CaretSource,
        line_index: usize,
    ) -> Self {
        Self {
            position,
            x,
            y,
            quality,
            source,
            line_index,
        }
    }
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
    pub fn line_index(self) -> usize {
        self.line_index
    }
}

/// A fixed-baseline layout of explicit lines, retaining empty/trailing lines.
///
/// Horizontal lines start at `(0, line_index * line_height)`; TTB columns start
/// at `(-line_index * line_height, 0)`. Horizontal RTL retains the engine's
/// right-edge origin and negative-x flow. There is no alignment or paragraph
/// direction detection. Ligature caret approximation and unsupported mappings
/// are inherited from [`CaretLayout`].
#[derive(Debug, Clone)]
pub struct HardLineLayout {
    text: String,
    lines: Vec<HardLine>,
    direction: TextDirection,
    line_height: f32,
}
impl HardLineLayout {
    /// Shapes each LF/CRLF-delimited slice using the same engine. The fixed
    /// step is `options.line_height.unwrap_or(options.font_size)`.
    /// Bare CR, tabs, other line/control characters and BiDi formatting controls
    /// are rejected, not silently normalized. Empty input produces one line.
    pub fn shape(engine: &FontEngine<'_>, text: &str) -> Result<Self, Error> {
        let options = engine.options();
        let line_height = options.line_height.unwrap_or(options.font_size);
        let direction = options.text_direction;
        Self::shape_with(text, direction, line_height, |slice| {
            engine.shape_mapped(slice)
        })
    }

    /// Shapes each LF/CRLF-delimited slice using [`FontFamily`] fallback.
    /// The fixed step is `options.line_height.unwrap_or(options.font_size)`.
    pub fn shape_family(
        family: &FontFamily,
        text: &str,
        options: FontOptions<'_>,
    ) -> Result<Self, Error> {
        let line_height = options.line_height.unwrap_or(options.font_size);
        let direction = options.text_direction;
        Self::shape_with(text, direction, line_height, |slice| {
            family.shape_mapped(slice, options.clone())
        })
    }

    /// Shapes each LF/CRLF-delimited horizontal LTR slice using [`CompositeFont`].
    /// The fixed step is `composite.options().line_height.unwrap_or(composite.options().font_size)`.
    pub fn shape_composite_ltr(composite: &CompositeFont<'_>, text: &str) -> Result<Self, Error> {
        let options = composite.options();
        let line_height = options.line_height.unwrap_or(options.font_size);
        Self::shape_with(text, TextDirection::LeftToRight, line_height, |slice| {
            composite.shape_mapped_ltr(slice)
        })
    }

    fn shape_with<F>(
        text: &str,
        direction: TextDirection,
        line_height: f32,
        mut shape_line: F,
    ) -> Result<Self, Error>
    where
        F: FnMut(&str) -> Result<ShapedText, Error>,
    {
        // Validate font and options even when no visible glyph is requested.
        shape_line("")?;
        if !line_height.is_finite() || line_height <= 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "line height must be positive and finite",
            ));
        }
        let ranges = split_hard_lines(text);
        let mut lines = Vec::with_capacity(ranges.len());
        for (index, (content, line_break)) in ranges.into_iter().enumerate() {
            let distance = index as f32 * line_height;
            if !distance.is_finite() {
                return Err(Error::new(ErrorKind::InvalidInput, "line origin overflow"));
            }
            let (origin_x, origin_y) = if direction == TextDirection::TopToBottom {
                (-distance, 0.0)
            } else {
                (0.0, distance)
            };
            let shaped = shape_line(&text[content.clone()])?;
            let carets = CaretLayout::new(&shaped)?;
            if shaped
                .glyph_run()
                .glyphs
                .iter()
                .any(|glyph| !(glyph.x + origin_x).is_finite() || !(glyph.y + origin_y).is_finite())
            {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "translated glyph origin overflow",
                ));
            }
            lines.push(HardLine {
                content,
                line_break,
                shaped,
                carets,
                origin_x,
                origin_y,
            });
        }
        Ok(Self {
            text: text.to_owned(),
            lines,
            direction,
            line_height,
        })
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn lines(&self) -> &[HardLine] {
        &self.lines
    }
    pub fn direction(&self) -> TextDirection {
        self.direction
    }
    /// Baseline spacing, not a font-derived line-box height.
    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    /// Resolves a global UTF-8 EGC boundary. Before and after a line break are
    /// distinct positions; the interior of CRLF is invalid. Affinity is retained
    /// but does not change geometry, since no soft wrap or BiDi is performed.
    pub fn caret_positions(&self, position: TextPosition) -> Result<Vec<LineCaretPosition>, Error> {
        let offset = position.byte_offset();
        let (index, line) = self
            .lines
            .iter()
            .enumerate()
            .find(|(_, line)| offset >= line.content.start && offset <= line.content.end)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    "not an available line-content boundary",
                )
            })?;
        let local = TextPosition::new(offset - line.content.start, position.affinity());
        line.carets
            .caret_positions(local)?
            .into_iter()
            .map(|caret| self.global_caret(index, caret))
            .collect()
    }

    /// Picks the nearest baseline on the cross axis, then the nearest inline
    /// caret on that line. Outside points clamp; exact baseline ties choose the
    /// earlier logical line. This is not ink containment or line-box hit testing.
    /// Both coordinates must be finite. Inline tie/affinity rules match CaretLayout.
    pub fn hit_test(&self, x: f32, y: f32) -> Result<LineCaretPosition, Error> {
        if !x.is_finite() || !y.is_finite() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "hit-test coordinates must be finite",
            ));
        }
        let vertical = self.direction == TextDirection::TopToBottom;
        let origins: Vec<(f32, f32)> = self
            .lines
            .iter()
            .map(|line| (line.origin_x, line.origin_y))
            .collect();
        let best_index = select_nearest_line_index(&origins, vertical, x, y);
        let line = &self.lines[best_index];
        // Cross-axis distance has already been considered. Pass zero on that
        // axis so subtracting extreme, finite inputs cannot overflow an f32.
        let local = if vertical {
            line.carets.hit_test(0.0, y)?
        } else {
            line.carets.hit_test(x, 0.0)?
        };
        self.global_caret(best_index, local)
    }

    fn global_caret(&self, index: usize, caret: CaretPosition) -> Result<LineCaretPosition, Error> {
        let line = &self.lines[index];
        let x = caret.x() + line.origin_x;
        let y = caret.y() + line.origin_y;
        if !x.is_finite() || !y.is_finite() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "translated caret overflow",
            ));
        }
        Ok(LineCaretPosition::new(
            TextPosition::new(
                line.content.start + caret.text_position().byte_offset(),
                caret.text_position().affinity(),
            ),
            x,
            y,
            caret.quality(),
            caret.source(),
            index,
        ))
    }
}

pub(crate) fn split_hard_lines(text: &str) -> Vec<(Range<usize>, Range<usize>)> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for (index, ch) in text.char_indices() {
        if ch == '\n' {
            let content_end = if index > start && text.as_bytes()[index - 1] == b'\r' {
                index - 1
            } else {
                index
            };
            ranges.push((start..content_end, content_end..index + 1));
            start = index + 1;
        }
    }
    ranges.push((start..text.len(), text.len()..text.len()));
    ranges
}

pub(crate) fn select_nearest_line_index(
    origins: &[(f32, f32)],
    vertical: bool,
    x: f32,
    y: f32,
) -> usize {
    let cross = if vertical { -(x as f64) } else { y as f64 };
    // Compare monotone midpoints rather than subtracting a remote point
    // from each baseline: even f64 loses small differences near f32::MAX.
    // <= retains the earlier logical line on exact ties.
    let mut best_index = origins.len() - 1;
    for (index, pair) in origins.windows(2).enumerate() {
        let first = if vertical {
            -(pair[0].0 as f64)
        } else {
            pair[0].1 as f64
        };
        let next = if vertical {
            -(pair[1].0 as f64)
        } else {
            pair[1].1 as f64
        };
        if cross <= first + (next - first) / 2.0 {
            best_index = index;
            break;
        }
    }
    // If f32 rounding makes adjacent origins coincide, preserve the
    // documented earliest-line tie rule, including outside clamping.
    while best_index > 0 && origins[best_index - 1] == origins[best_index] {
        best_index -= 1;
    }
    best_index
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CaretAffinity, FontFile};
    const FONT: &[u8] = include_bytes!("../tests/fixtures/european-fallback.ttc");
    fn pos(byte: usize) -> TextPosition {
        TextPosition::new(byte, CaretAffinity::Downstream)
    }
    #[test]
    fn global_utf8_crlf_and_rendering_ranges() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(1).unwrap();
        let engine = face
            .engine()
            .with_font_size(1000.0)
            .with_line_height(1200.0);
        let text = "Bß\nfi\r\na\u{308}\n";
        let layout = HardLineLayout::shape(&engine, text).unwrap();
        assert_eq!(layout.text(), text);
        let ranges: Vec<_> = layout
            .lines()
            .iter()
            .map(|line| (line.content_range(), line.break_range()))
            .collect();
        assert_eq!(
            ranges,
            vec![
                (0..3, 3..4),
                (4..6, 6..8),
                (8..11, 11..12),
                (12..12, 12..12)
            ]
        );
        for (index, line) in layout.lines().iter().enumerate() {
            assert_eq!(line.origin_y(), index as f32 * 1200.0);
            assert_eq!(
                format!("{:?}", line.shaped().glyph_run()),
                format!("{:?}", engine.shape(&text[line.content_range()]).unwrap())
            );
        }
        for byte in [0, 1, 3, 4, 5, 6, 8, 11, 12] {
            assert!(layout.caret_positions(pos(byte)).is_ok(), "{byte}");
        }
        for byte in [2, 7, 9, 10, 13] {
            assert!(layout.caret_positions(pos(byte)).is_err(), "{byte}");
        }
        assert_eq!(layout.caret_positions(pos(6)).unwrap()[0].line_index(), 1);
        assert_eq!(layout.caret_positions(pos(8)).unwrap()[0].line_index(), 2);
        assert_eq!(layout.caret_positions(pos(12)).unwrap()[0].y(), 3600.0);
    }
    #[test]
    fn empty_consecutive_and_trailing_lines() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(1).unwrap();
        let engine = face.engine().with_font_size(10.0);
        for (text, count) in [("", 1), ("\n", 2), ("\r\n\n", 3), ("B\n\nB\n", 4)] {
            let layout = HardLineLayout::shape(&engine, text).unwrap();
            assert_eq!(layout.lines().len(), count);
            assert_eq!(layout.line_height(), 10.0);
            assert_eq!(
                layout.caret_positions(pos(text.len())).unwrap()[0].line_index(),
                count - 1
            );
        }
    }
    #[test]
    fn direction_axes_hit_testing_clamp_and_ties() {
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
                .with_font_size(1000.0)
                .with_line_height(1200.0);
            let engine = face.engine().with_options(options);
            let layout = HardLineLayout::shape(&engine, "BB\nB\n").unwrap();
            let translated = layout
                .lines()
                .iter()
                .flat_map(|line| {
                    line.shaped()
                        .glyph_run()
                        .glyphs
                        .iter()
                        .cloned()
                        .map(|mut glyph| {
                            glyph.x += line.origin_x();
                            glyph.y += line.origin_y();
                            glyph
                        })
                })
                .collect();
            assert_eq!(
                format!("{:?}", crate::GlyphRun::new(translated)),
                format!("{:?}", engine.shape("BB\nB\n").unwrap())
            );
            let caret = layout.caret_positions(pos(4)).unwrap()[0];
            let vertical = direction == TextDirection::TopToBottom;
            assert_eq!(
                if vertical { caret.x() } else { caret.y() },
                if vertical { -1200.0 } else { 1200.0 }
            );
            assert_eq!(
                layout
                    .hit_test(caret.x(), caret.y())
                    .unwrap()
                    .text_position()
                    .byte_offset(),
                4
            );
            let tie = if vertical {
                layout.hit_test(-600.0, 0.0)
            } else {
                layout.hit_test(0.0, 600.0)
            }
            .unwrap();
            assert_eq!(tie.line_index(), 0);
            let far = if vertical {
                layout.hit_test(-f32::MAX, f32::MAX)
            } else {
                layout.hit_test(f32::MAX, f32::MAX)
            }
            .unwrap();
            assert_eq!(far.line_index(), 2);
            let before = if vertical {
                layout.hit_test(f32::MAX, 0.0)
            } else {
                layout.hit_test(0.0, -f32::MAX)
            }
            .unwrap();
            assert_eq!(before.line_index(), 0);
            let inline_end = if vertical {
                layout.hit_test(0.0, f32::MAX)
            } else if direction == TextDirection::RightToLeft {
                layout.hit_test(-f32::MAX, 0.0)
            } else {
                layout.hit_test(f32::MAX, 0.0)
            }
            .unwrap();
            assert_eq!(inline_end.text_position().byte_offset(), 2);
            let inline_start = if vertical {
                layout.hit_test(0.0, -f32::MAX)
            } else if direction == TextDirection::RightToLeft {
                layout.hit_test(f32::MAX, 0.0)
            } else {
                layout.hit_test(-f32::MAX, 0.0)
            }
            .unwrap();
            assert_eq!(inline_start.text_position().byte_offset(), 0);
        }
    }
    #[test]
    fn unsupported_and_invalid_inputs() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(1).unwrap();
        let engine = face.engine();
        for text in [
            "B\rB",
            "\r",
            "B\tB",
            "\u{85}",
            "\u{2028}",
            "\u{2029}",
            "B\u{202e}B",
            "\r\r\n",
        ] {
            assert!(HardLineLayout::shape(&engine, text).is_err(), "{text:?}");
        }
        for height in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(HardLineLayout::shape(&face.engine().with_line_height(height), "").is_err());
        }
        assert!(HardLineLayout::shape(&face.engine().with_font_size(0.0), "").is_err());
        assert!(HardLineLayout::shape(&face.engine().with_line_height(f32::MAX), "\n\n").is_err());
        let empty = HardLineLayout::shape(&engine, "").unwrap();
        assert!(empty.hit_test(f32::NAN, 0.0).is_err());
        assert!(empty.hit_test(0.0, f32::INFINITY).is_err());
        assert!(empty.caret_positions(pos(1)).is_err());
        let upstream = TextPosition::new(0, CaretAffinity::Upstream);
        assert_eq!(
            empty.caret_positions(upstream).unwrap()[0].text_position(),
            upstream
        );
    }
}
