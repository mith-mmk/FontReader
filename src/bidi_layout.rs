//! Bounded horizontal UAX #9 layout with hard LF/CRLF lines.
//! Unicode 16.0 BiDi levels/order are resolved on original text before controls
//! are hidden. Shaping retains logical order; visual runs are translated, never
//! reversed. No wrapping, vertical layout, overrides, or fallback is performed.
use crate::caret::{authored_carets, CaretSource};
use crate::{CaretAffinity, CaretQuality, FontEngine, ShapedText, TextDirection, TextPosition};
use std::io::{Error, ErrorKind};
use std::ops::Range;
use unicode_bidi::{BidiInfo, Level};
use unicode_segmentation::UnicodeSegmentation;

/// Paragraph base direction. Auto uses UAX #9 first-strong detection per hard line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BidiBaseDirection {
    Auto,
    LeftToRight,
    RightToLeft,
}

/// One logical source slice, positioned in visual order within its line.
#[derive(Debug, Clone)]
pub struct BidiVisualRun {
    range: Range<usize>,
    level: u8,
    shaped: ShapedText,
    origin_x: f32,
    origin_y: f32,
}
impl BidiVisualRun {
    pub fn byte_range(&self) -> Range<usize> {
        self.range.clone()
    }
    pub fn level(&self) -> u8 {
        self.level
    }
    /// Source ranges are run-local. Add `byte_range().start` for global offsets.
    /// Render the glyph run translated by `origin_x(), origin_y()`.
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

#[derive(Debug, Clone)]
pub struct BidiLine {
    range: Range<usize>,
    line_break: Range<usize>,
    runs: Vec<BidiVisualRun>,
    base_level: u8,
    width: f32,
    baseline: f32,
}
impl BidiLine {
    pub fn content_range(&self) -> Range<usize> {
        self.range.clone()
    }
    pub fn break_range(&self) -> Range<usize> {
        self.line_break.clone()
    }
    pub fn visual_runs(&self) -> &[BidiVisualRun] {
        &self.runs
    }
    pub fn base_level(&self) -> u8 {
        self.base_level
    }
    pub fn width(&self) -> f32 {
        self.width
    }
    pub fn baseline_y(&self) -> f32 {
        self.baseline
    }
}

/// A source boundary with its visual location. At directional boundaries,
/// Upstream follows preceding logical content; Downstream follows subsequent
/// logical content. They may have different x coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BidiCaretPosition {
    position: TextPosition,
    x: f32,
    y: f32,
    line_index: usize,
    quality: CaretQuality,
    source: CaretSource,
}
impl BidiCaretPosition {
    pub fn text_position(self) -> TextPosition {
        self.position
    }
    pub fn x(self) -> f32 {
        self.x
    }
    pub fn y(self) -> f32 {
        self.y
    }
    pub fn line_index(self) -> usize {
        self.line_index
    }
    pub fn quality(self) -> CaretQuality {
        self.quality
    }
    /// Authoritative provenance; legacy `quality()` stays conservative for internal stops.
    pub fn source(self) -> CaretSource {
        self.source
    }
}

/// Horizontal hard-line layout. Every line has left edge x=0, even when its
/// base direction is RTL. Fixed baseline spacing comes from the engine options.
/// Formatting controls remain editable zero-width source boundaries. LRO/RLO,
/// tabs, other separators, split EGCs, and unsupported shaping mappings error.
/// Ligature-internal stops prefer validated GDEF caret data for a single owning
/// glyph, otherwise use explicit approximation. `source()` identifies which;
/// legacy `quality()` remains Approximate for both kinds of internal stop.
/// With `layout`, contextual joining that crosses a level-run boundary is
/// rejected rather than silently shaping isolated forms (for example Arabic
/// letters separated by LRM). Isolates remain genuine joining barriers.
/// Arabic joining/GSUB requires the existing `layout` feature. This API does not
/// expand the engine's script/mark support or claim general shaping conformance.
#[derive(Debug, Clone)]
pub struct BidiParagraphLayout {
    text: String,
    lines: Vec<BidiLine>,
    carets: Vec<BidiCaretPosition>,
    line_height: f32,
}
impl BidiParagraphLayout {
    pub fn shape(
        engine: &FontEngine<'_>,
        text: &str,
        base: BidiBaseDirection,
    ) -> Result<Self, Error> {
        engine.shape_mapped("")?;
        if engine.options().text_direction == TextDirection::TopToBottom {
            return Err(unsupported("BiDi paragraph layout is horizontal only"));
        }
        let options = engine.options();
        let line_height = options.line_height.unwrap_or(options.font_size);
        let mut ranges = Vec::new();
        let mut start = 0;
        for (index, ch) in text.char_indices() {
            if ch == '\n' {
                let end = if index > start && text.as_bytes()[index - 1] == b'\r' {
                    index - 1
                } else {
                    index
                };
                ranges.push((start..end, end..index + 1));
                start = index + 1;
            }
        }
        ranges.push((start..text.len(), text.len()..text.len()));
        let mut lines = Vec::new();
        let mut carets = Vec::new();
        for (line_index, (range, line_break)) in ranges.into_iter().enumerate() {
            let line_text = &text[range.clone()];
            if line_text.chars().any(|c| {
                c.is_control() || matches!(c, '\u{2028}' | '\u{2029}' | '\u{202d}' | '\u{202e}')
            }) {
                return Err(unsupported(
                    "only LF/CRLF lines without tabs or directional overrides are supported",
                ));
            }
            let baseline = line_index as f32 * line_height;
            if !baseline.is_finite() {
                return Err(invalid("line baseline overflow"));
            }
            let level = match base {
                BidiBaseDirection::Auto => None,
                BidiBaseDirection::LeftToRight => Some(Level::ltr()),
                BidiBaseDirection::RightToLeft => Some(Level::rtl()),
            };
            // Analysis must see controls; filtering them beforehand changes UAX #9.
            let bidi = BidiInfo::new(line_text, level);
            let mut runs = Vec::new();
            let mut width = 0.0f32;
            let mut base_level = level.unwrap_or(Level::ltr()).number();
            let mut boundaries = vec![0];
            boundaries.extend(line_text.grapheme_indices(true).map(|(i, g)| i + g.len()));
            if let Some(paragraph) = bidi.paragraphs.first() {
                if bidi.paragraphs.len() != 1 {
                    return Err(unsupported("unexpected paragraph separator"));
                }
                base_level = paragraph.level.number();
                let (levels, visual) = bidi.visual_runs(paragraph, 0..line_text.len());
                #[cfg(feature = "layout")]
                validate_joining_boundaries(line_text, &visual)?;
                for local in visual {
                    if boundaries.binary_search(&local.start).is_err()
                        || boundaries.binary_search(&local.end).is_err()
                    {
                        return Err(unsupported("BiDi level run splits an extended grapheme"));
                    }
                    let run_level = levels[local.start];
                    let direction = if run_level.is_rtl() {
                        TextDirection::RightToLeft
                    } else {
                        TextDirection::LeftToRight
                    };
                    let shaped =
                        engine.shape_bidi_run_mapped(&line_text[local.clone()], direction)?;
                    let (stops, advance) = run_stops(&shaped)?;
                    let origin_x = if run_level.is_rtl() {
                        width + advance
                    } else {
                        width
                    };
                    let next_width = width + advance;
                    if !origin_x.is_finite() || !next_width.is_finite() {
                        return Err(invalid("visual run advance overflow"));
                    }
                    if shaped
                        .glyph_run()
                        .glyphs
                        .iter()
                        .any(|g| !(g.x + origin_x).is_finite() || !(g.y + baseline).is_finite())
                    {
                        return Err(invalid("translated glyph origin overflow"));
                    }
                    for (offset, inline, quality, source) in stops {
                        let x = origin_x + if run_level.is_rtl() { -inline } else { inline };
                        if !x.is_finite() {
                            return Err(invalid("caret coordinate overflow"));
                        }
                        let global = range.start + local.start + offset;
                        // Run edges select their own logical side; internal
                        // boundaries support either affinity at the same point.
                        for affinity in [CaretAffinity::Upstream, CaretAffinity::Downstream] {
                            if (offset == 0
                                && affinity == CaretAffinity::Upstream
                                && local.start != 0)
                                || (offset == shaped.text().len()
                                    && affinity == CaretAffinity::Downstream
                                    && local.end != line_text.len())
                            {
                                continue;
                            }
                            carets.push(BidiCaretPosition {
                                position: TextPosition::new(global, affinity),
                                x,
                                y: baseline,
                                line_index,
                                quality,
                                source,
                            });
                        }
                    }
                    runs.push(BidiVisualRun {
                        range: range.start + local.start..range.start + local.end,
                        level: run_level.number(),
                        shaped,
                        origin_x,
                        origin_y: baseline,
                    });
                    width = next_width;
                }
            }
            if line_text.is_empty() {
                for affinity in [CaretAffinity::Upstream, CaretAffinity::Downstream] {
                    carets.push(BidiCaretPosition {
                        position: TextPosition::new(range.start, affinity),
                        x: 0.0,
                        y: baseline,
                        line_index,
                        quality: CaretQuality::AdvanceBoundary,
                        source: CaretSource::Advance,
                    });
                }
            }
            lines.push(BidiLine {
                range,
                line_break,
                runs,
                base_level,
                width,
                baseline,
            });
        }
        Ok(Self {
            text: text.to_owned(),
            lines,
            carets,
            line_height,
        })
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn lines(&self) -> &[BidiLine] {
        &self.lines
    }
    pub fn line_height(&self) -> f32 {
        self.line_height
    }
    /// Returns every visual side of this original EGC byte boundary, carrying
    /// its affinity. Same-coordinate upstream/downstream entries are retained.
    pub fn visual_carets(&self, byte_offset: usize) -> Result<Vec<BidiCaretPosition>, Error> {
        let mut found: Vec<_> = self
            .carets
            .iter()
            .copied()
            .filter(|c| c.position.byte_offset() == byte_offset)
            .collect();
        found.sort_by_key(|c| affinity_order(c.position.affinity()));
        if found.is_empty() {
            return Err(invalid("not an available grapheme boundary"));
        }
        Ok(found)
    }
    pub fn caret_positions(&self, position: TextPosition) -> Result<Vec<BidiCaretPosition>, Error> {
        let found: Vec<_> = self
            .visual_carets(position.byte_offset())?
            .into_iter()
            .filter(|c| c.position.affinity() == position.affinity())
            .collect();
        if found.is_empty() {
            return Err(invalid("affinity has no visual position"));
        }
        Ok(found)
    }
    /// Nearest baseline, then nearest inline stop. Outside coordinates clamp.
    /// Exact ties select lower source byte, then Downstream affinity. No ink
    /// containment or typographic line box is inferred.
    pub fn hit_test(&self, x: f32, y: f32) -> Result<BidiCaretPosition, Error> {
        if !x.is_finite() || !y.is_finite() {
            return Err(invalid("coordinates must be finite"));
        }
        let mut line_index = self.lines.len() - 1;
        for (i, pair) in self.lines.windows(2).enumerate() {
            let midpoint =
                pair[0].baseline as f64 + (pair[1].baseline as f64 - pair[0].baseline as f64) / 2.0;
            if y as f64 <= midpoint {
                line_index = i;
                break;
            }
        }
        while line_index > 0
            && self.lines[line_index - 1].baseline == self.lines[line_index].baseline
        {
            line_index -= 1;
        }
        let mut stops: Vec<_> = self
            .carets
            .iter()
            .copied()
            .filter(|c| c.line_index == line_index)
            .collect();
        stops.sort_by_key(|c| {
            (
                c.position.byte_offset(),
                affinity_order(c.position.affinity()),
            )
        });
        let min = stops.iter().map(|c| c.x).fold(f32::INFINITY, f32::min);
        let max = stops.iter().map(|c| c.x).fold(f32::NEG_INFINITY, f32::max);
        let x = x.clamp(min, max) as f64;
        let mut best = stops[0];
        let mut distance = f64::INFINITY;
        for stop in stops {
            let delta = (stop.x as f64 - x).abs();
            if delta < distance {
                best = stop;
                distance = delta;
            }
        }
        Ok(best)
    }
}
// UAX #9 levels determine visual placement, not contextual joining boundaries.
// For example LRM is joining-transparent but can occupy a separate LTR run
// between Arabic letters. Independent run shaping would incorrectly isolate
// those letters. Until the engine accepts cross-run joining context, reject
// any partition that changes the existing joining analyzer's forms. Isolates
// have joining type U, so genuine isolate barriers remain supported.
#[cfg(feature = "layout")]
fn validate_joining_boundaries(text: &str, runs: &[Range<usize>]) -> Result<(), Error> {
    let chars: Vec<_> = text.chars().collect();
    let full_forms = crate::unicode::joining::forms(&chars);
    let mut offsets: Vec<_> = text.char_indices().map(|(index, _)| index).collect();
    offsets.push(text.len());
    for run in runs {
        let start = offsets
            .binary_search(&run.start)
            .map_err(|_| unsupported("BiDi run splits a scalar"))?;
        let end = offsets
            .binary_search(&run.end)
            .map_err(|_| unsupported("BiDi run splits a scalar"))?;
        let local_forms = crate::unicode::joining::forms(&chars[start..end]);
        if full_forms[start..end]
            .iter()
            .zip(&local_forms)
            .any(|(expected, actual)| expected.is_some() && expected != actual)
        {
            return Err(unsupported(
                "contextual joining crosses a BiDi level-run boundary",
            ));
        }
    }
    Ok(())
}

fn affinity_order(affinity: CaretAffinity) -> u8 {
    if affinity == CaretAffinity::Downstream {
        0
    } else {
        1
    }
}
fn unsupported(message: &str) -> Error {
    Error::new(ErrorKind::Unsupported, message)
}
fn invalid(message: &str) -> Error {
    Error::new(ErrorKind::InvalidInput, message)
}
fn control(ch: char) -> bool {
    crate::fontreader::Font::is_bidi_format_control(ch)
}

// Logical, positive-advance stops. Invisible controls retain zero-width source
// positions. Any other missing ownership is rejected, never guessed.
type RunStop = (usize, f32, CaretQuality, CaretSource);

fn run_stops(shaped: &ShapedText) -> Result<(Vec<RunStop>, f32), Error> {
    let text = shaped.text();
    let graphemes: Vec<_> = text
        .grapheme_indices(true)
        .map(|(i, g)| (i..i + g.len(), g.chars().all(control)))
        .collect();
    let mut boundaries = vec![0];
    boundaries.extend(graphemes.iter().map(|(r, _)| r.end));
    let mut groups: Vec<(Range<usize>, f32, Option<usize>)> = Vec::new();
    for (glyph_index, (glyph, source)) in shaped
        .glyph_run()
        .glyphs
        .iter()
        .zip(shaped.glyph_sources())
        .enumerate()
    {
        let advance = glyph.glyph.metrics.advance_x;
        if !advance.is_finite() || advance < 0.0 || glyph.glyph.metrics.advance_y != 0.0 {
            return Err(unsupported(
                "BiDi carets require nonnegative finite horizontal advances",
            ));
        }
        let range = source.byte_range();
        if range.is_empty() {
            return Err(unsupported("empty source ownership"));
        }
        if let Some((last, width, owner)) = groups.last_mut() {
            if range.start < last.start {
                return Err(unsupported("reordered source mapping"));
            }
            if range.start < last.end {
                last.end = last.end.max(range.end);
                *width += advance;
                *owner = None;
                if !width.is_finite() {
                    return Err(invalid("advance overflow"));
                }
                continue;
            }
        }
        groups.push((range, advance, Some(glyph_index)));
    }
    let mut stops = vec![(0, 0.0, CaretQuality::AdvanceBoundary, CaretSource::Advance)];
    let mut cursor = 0;
    let mut pen = 0.0;
    for (range, advance, owner) in groups {
        let start = boundaries
            .binary_search(&range.start)
            .map_err(|_| unsupported("partial grapheme ownership"))?;
        let end = boundaries
            .binary_search(&range.end)
            .map_err(|_| unsupported("partial grapheme ownership"))?;
        if start < cursor {
            return Err(unsupported("overlapping source groups"));
        }
        for (gap, hidden) in &graphemes[cursor..start] {
            if !hidden {
                return Err(unsupported("unowned visible source"));
            }
            stops.push((
                gap.end,
                pen,
                CaretQuality::AdvanceBoundary,
                CaretSource::Advance,
            ));
        }
        let visible = graphemes[start..end]
            .iter()
            .filter(|(_, hidden)| !hidden)
            .count();
        if visible == 0 {
            return Err(unsupported("visible glyph owns only formatting controls"));
        }
        let authored = owner.and_then(|index| authored_carets(shaped, index, visible, advance));
        let mut count = 0;
        for (grapheme, hidden) in &graphemes[start..end] {
            if !hidden {
                count += 1;
            }
            let (distance, source) = if count == 0 {
                (0.0, CaretSource::Advance)
            } else if count == visible {
                (advance, CaretSource::Advance)
            } else if let Some(authored) = authored {
                (authored[count - 1], CaretSource::FontAuthored)
            } else {
                (
                    advance * (count as f32 / visible as f32),
                    CaretSource::Approximate,
                )
            };
            let x = pen + distance;
            if !x.is_finite() {
                return Err(invalid("caret advance overflow"));
            }
            stops.push((
                grapheme.end,
                x,
                if count == visible {
                    CaretQuality::AdvanceBoundary
                } else {
                    CaretQuality::Approximate
                },
                source,
            ));
        }
        pen += advance;
        cursor = end;
    }
    for (gap, hidden) in &graphemes[cursor..] {
        if !hidden {
            return Err(unsupported("unowned visible source"));
        }
        stops.push((
            gap.end,
            pen,
            CaretQuality::AdvanceBoundary,
            CaretSource::Advance,
        ));
    }
    Ok((stops, pen))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FontFile;
    const FONT: &[u8] = include_bytes!("../tests/fixtures/bidi-layout.ttf");
    fn position(byte: usize, affinity: CaretAffinity) -> TextPosition {
        TextPosition::new(byte, affinity)
    }
    #[test]
    fn mixed_runs_have_dual_directional_carets() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(0).unwrap();
        let engine = face.engine().with_font_size(1000.0);
        let layout =
            BidiParagraphLayout::shape(&engine, "aאבb", BidiBaseDirection::LeftToRight).unwrap();
        assert_eq!(layout.lines()[0].width(), 2400.0);
        let runs = layout.lines()[0].visual_runs();
        assert_eq!(
            runs.iter().map(|r| r.byte_range()).collect::<Vec<_>>(),
            vec![0..1, 1..5, 5..6]
        );
        assert_eq!(
            runs.iter().map(|r| r.origin_x()).collect::<Vec<_>>(),
            vec![0.0, 1800.0, 1800.0]
        );
        let upstream = layout
            .caret_positions(position(1, CaretAffinity::Upstream))
            .unwrap()[0];
        let downstream = layout
            .caret_positions(position(1, CaretAffinity::Downstream))
            .unwrap()[0];
        assert_eq!((upstream.x(), downstream.x()), (600.0, 1800.0));
        assert_eq!(layout.visual_carets(1).unwrap().len(), 2);
        assert_eq!(
            layout
                .caret_positions(position(3, CaretAffinity::Downstream))
                .unwrap()[0]
                .x(),
            1200.0
        );
        assert!(layout.visual_carets(2).is_err());
        assert_eq!(
            layout
                .hit_test(f32::MAX, 0.0)
                .unwrap()
                .text_position()
                .byte_offset(),
            6
        );
        assert_eq!(
            layout
                .hit_test(-f32::MAX, 0.0)
                .unwrap()
                .text_position()
                .byte_offset(),
            0
        );
    }
    #[test]
    fn controls_are_analyzed_but_not_drawn_and_keep_positions() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(0).unwrap();
        let engine = face.engine().with_font_size(1000.0);
        for text in ["a\u{2067}אב\u{2069}b", "a\u{202b}אב\u{202c}b"] {
            let layout =
                BidiParagraphLayout::shape(&engine, text, BidiBaseDirection::LeftToRight).unwrap();
            assert_eq!(layout.lines()[0].width(), 2400.0);
            assert_eq!(
                layout.lines()[0]
                    .visual_runs()
                    .iter()
                    .map(|r| r.shaped().glyph_run().glyphs.len())
                    .sum::<usize>(),
                4
            );
            for (byte, _) in text.grapheme_indices(true) {
                assert!(layout.visual_carets(byte).is_ok(), "{byte}");
            }
            assert!(layout.visual_carets(text.len()).is_ok());
        }
        for text in ["\u{200e}\u{200f}", "\u{2067}\u{2069}", "\u{061c}"] {
            let layout =
                BidiParagraphLayout::shape(&engine, text, BidiBaseDirection::Auto).unwrap();
            assert_eq!(layout.lines()[0].width(), 0.0);
            assert!(layout.lines()[0].visual_runs().iter().all(|r| r
                .shaped()
                .glyph_run()
                .glyphs
                .is_empty()));
            for (byte, _) in text.grapheme_indices(true) {
                assert!(layout.visual_carets(byte).is_ok());
            }
            assert!(layout.visual_carets(text.len()).is_ok());
            assert_eq!(
                layout
                    .hit_test(f32::MAX, 0.0)
                    .unwrap()
                    .text_position()
                    .byte_offset(),
                0
            );
        }
    }
    #[test]
    fn mirrors_marks_and_ligatures_preserve_original_source() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(0).unwrap();
        let engine = face.engine().with_font_size(1000.0);
        let layout =
            BidiParagraphLayout::shape(&engine, "(אב)", BidiBaseDirection::RightToLeft).unwrap();
        let run = &layout.lines()[0].visual_runs()[0];
        assert_eq!(run.shaped().text(), "(אב)");
        assert_eq!(run.shaped().glyph_sources()[0].byte_range(), 0..1);
        let mirrored = engine.shape(")").unwrap();
        assert_eq!(
            format!("{:?}", run.shaped().glyph_run().glyphs[0].glyph),
            format!("{:?}", mirrored.glyphs[0].glyph)
        );
        let marks =
            BidiParagraphLayout::shape(&engine, "aא\u{05b0}b", BidiBaseDirection::Auto).unwrap();
        assert_eq!(marks.lines()[0].width(), 1800.0);
        assert!(marks.visual_carets(3).is_err());
        assert!(marks.visual_carets(1).is_ok());
        assert!(marks.visual_carets(5).is_ok());
        let liga =
            BidiParagraphLayout::shape(&engine, "fiאב", BidiBaseDirection::LeftToRight).unwrap();
        let internal = liga
            .caret_positions(position(1, CaretAffinity::Downstream))
            .unwrap()[0];
        if cfg!(feature = "layout") {
            assert_eq!(internal.x(), 450.0);
            assert_eq!(internal.quality(), CaretQuality::Approximate);
        } else {
            assert_eq!(internal.x(), 600.0);
            assert_eq!(internal.quality(), CaretQuality::AdvanceBoundary);
        }
    }
    #[test]
    fn hard_lines_base_direction_and_empty_text() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(0).unwrap();
        let engine = face
            .engine()
            .with_font_size(1000.0)
            .with_line_height(1200.0);
        let text = "אב\r\na\n\n";
        let layout = BidiParagraphLayout::shape(&engine, text, BidiBaseDirection::Auto).unwrap();
        assert_eq!(layout.lines().len(), 4);
        assert_eq!(layout.lines()[0].base_level(), 1);
        assert_eq!(layout.lines()[1].base_level(), 0);
        assert_eq!(layout.lines()[0].content_range(), 0..4);
        assert_eq!(layout.lines()[0].break_range(), 4..6);
        assert!(layout.visual_carets(5).is_err());
        assert!(layout.visual_carets(4).is_ok());
        assert!(layout.visual_carets(6).is_ok());
        assert_eq!(
            layout
                .hit_test(f32::MAX, f32::MAX)
                .unwrap()
                .text_position()
                .byte_offset(),
            text.len()
        );
        assert_eq!(layout.hit_test(0.0, 600.0).unwrap().line_index(), 0);
        let empty =
            BidiParagraphLayout::shape(&engine, "", BidiBaseDirection::RightToLeft).unwrap();
        assert_eq!(empty.lines()[0].base_level(), 1);
        assert_eq!(
            empty
                .hit_test(10.0, 10.0)
                .unwrap()
                .text_position()
                .byte_offset(),
            0
        );
    }
    #[test]
    fn unsupported_modes_and_nonfinite_coordinates_fail() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(0).unwrap();
        let engine = face.engine();
        for text in ["\r", "\t", "\u{2028}", "\u{2029}", "\u{202d}a", "\u{202e}a"] {
            assert!(
                BidiParagraphLayout::shape(&engine, text, BidiBaseDirection::Auto).is_err(),
                "{text:?}"
            );
        }
        let vertical = engine.clone().with_options(
            engine
                .options()
                .with_text_direction(TextDirection::TopToBottom),
        );
        assert!(BidiParagraphLayout::shape(&vertical, "a", BidiBaseDirection::Auto).is_err());
        assert!(BidiParagraphLayout::shape(
            &engine.clone().with_line_height(f32::MAX),
            "\n\n",
            BidiBaseDirection::Auto
        )
        .is_err());
        let layout = BidiParagraphLayout::shape(&engine, "a", BidiBaseDirection::Auto).unwrap();
        assert!(layout.hit_test(f32::NAN, 0.0).is_err());
        assert!(layout.hit_test(0.0, f32::INFINITY).is_err());
        assert!(layout.visual_carets(2).is_err());
    }
    #[test]
    fn numeric_visual_order_and_empty_isolate_joining() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(0).unwrap();
        let engine = face.engine().with_font_size(1000.0);
        let numbers = BidiParagraphLayout::shape(&engine, "אב12", BidiBaseDirection::Auto).unwrap();
        assert_eq!(
            numbers.lines()[0]
                .visual_runs()
                .iter()
                .map(|r| r.byte_range())
                .collect::<Vec<_>>(),
            vec![4..6, 0..4]
        );
        assert_eq!(numbers.lines()[0].visual_runs()[0].shaped().text(), "12");
        assert_eq!(numbers.lines()[0].width(), 2400.0);
        let text = "ب\u{2066}\u{2069}ب";
        let isolated =
            BidiParagraphLayout::shape(&engine, text, BidiBaseDirection::RightToLeft).unwrap();
        let expected = engine.clone().with_right_to_left().shape("ب").unwrap();
        let glyphs: Vec<_> = isolated.lines()[0]
            .visual_runs()
            .iter()
            .flat_map(|r| r.shaped().glyph_run().glyphs.iter())
            .collect();
        assert_eq!(glyphs.len(), 2);
        for glyph in glyphs {
            assert_eq!(
                format!("{:?}", glyph.glyph),
                format!("{:?}", expected.glyphs[0].glyph)
            );
        }
        for (byte, _) in text.grapheme_indices(true) {
            assert!(isolated.visual_carets(byte).is_ok());
        }
    }
    #[test]
    fn leading_trailing_and_internal_controls_are_zero_width() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(0).unwrap();
        let engine = face.engine().with_font_size(1000.0);
        for text in ["\u{200e}fi\u{200e}", "f\u{200e}i"] {
            let layout =
                BidiParagraphLayout::shape(&engine, text, BidiBaseDirection::LeftToRight).unwrap();
            let expected_width = if cfg!(feature = "layout") {
                900.0
            } else {
                1200.0
            };
            assert_eq!(layout.lines()[0].width(), expected_width);
            for (start, ch) in text.char_indices().filter(|(_, ch)| control(*ch)) {
                let before = layout
                    .caret_positions(position(start, CaretAffinity::Downstream))
                    .unwrap()[0];
                let after = layout
                    .caret_positions(position(start + ch.len_utf8(), CaretAffinity::Upstream))
                    .unwrap()[0];
                assert_eq!(before.x(), after.x());
            }
            assert_eq!(layout.hit_test(f32::MAX, 0.0).unwrap().x(), expected_width);
        }
    }
    #[test]
    fn cross_level_joining_is_explicitly_unsupported() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(0).unwrap();
        let engine = face.engine().with_font_size(1000.0);
        let result =
            BidiParagraphLayout::shape(&engine, "ب\u{200e}ب", BidiBaseDirection::RightToLeft);
        if cfg!(feature = "layout") {
            let error = result.unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Unsupported);
            assert!(error.to_string().contains("contextual joining crosses"));
        } else {
            // Without layout, contextual joining is already absent by contract.
            assert!(result.is_ok());
        }
        for text in ["بب", "ب\u{200c}ب", "ب\u{2066}\u{2069}ب"] {
            assert!(
                BidiParagraphLayout::shape(&engine, text, BidiBaseDirection::RightToLeft).is_ok(),
                "{text:?}"
            );
        }
        // Transparent controls that do not split the level run still join.
        let joined = engine.clone().with_right_to_left().shape("بب").unwrap();
        for control in ['\u{200f}', '\u{061c}'] {
            let text = format!("ب{control}ب");
            let layout =
                BidiParagraphLayout::shape(&engine, &text, BidiBaseDirection::RightToLeft).unwrap();
            let glyphs: Vec<_> = layout.lines()[0]
                .visual_runs()
                .iter()
                .flat_map(|run| run.shaped().glyph_run().glyphs.iter())
                .collect();
            assert_eq!(glyphs.len(), 2);
            for (actual, expected) in glyphs.iter().zip(&joined.glyphs) {
                assert_eq!(
                    format!("{:?}", actual.glyph.layers),
                    format!("{:?}", expected.glyph.layers)
                );
            }
        }
    }
    #[test]
    fn authored_control_plateaus_preserve_provenance() {
        let file = FontFile::from_buffer(FONT).unwrap();
        let face = file.face(0).unwrap();
        let mut run = face.engine().shape("a").unwrap();
        run.glyphs[0].glyph.metrics.advance_x = 800.0;
        let text = "f\u{200e}i";
        let shaped = ShapedText::new(
            text,
            run.clone(),
            vec![crate::GlyphSource::new(0..text.len(), 0, 0)],
            TextDirection::LeftToRight,
        )
        .with_ligature_carets(vec![Some(vec![200.0])]);
        let (stops, width) = run_stops(&shaped).unwrap();
        assert_eq!(width, 800.0);
        assert_eq!(
            stops,
            vec![
                (0, 0.0, CaretQuality::AdvanceBoundary, CaretSource::Advance),
                (
                    1,
                    200.0,
                    CaretQuality::Approximate,
                    CaretSource::FontAuthored
                ),
                (
                    4,
                    200.0,
                    CaretQuality::Approximate,
                    CaretSource::FontAuthored
                ),
                (
                    5,
                    800.0,
                    CaretQuality::AdvanceBoundary,
                    CaretSource::Advance
                )
            ]
        );
        let invalid = ShapedText::new(
            text,
            run,
            vec![crate::GlyphSource::new(0..text.len(), 0, 0)],
            TextDirection::LeftToRight,
        )
        .with_ligature_carets(vec![Some(vec![900.0])]);
        let (stops, _) = run_stops(&invalid).unwrap();
        assert_eq!((stops[1].1, stops[1].3), (400.0, CaretSource::Approximate));
    }
    #[cfg(feature = "layout")]
    #[test]
    fn paragraph_propagates_font_authored_caret_source() {
        let file =
            FontFile::from_buffer(include_bytes!("../tests/fixtures/gdef-carets.ttc")).unwrap();
        let face = file.face(0).unwrap();
        let engine = face.engine().with_font_size(1000.0);
        let layout =
            BidiParagraphLayout::shape(&engine, "fi", BidiBaseDirection::LeftToRight).unwrap();
        let caret = layout
            .caret_positions(position(1, CaretAffinity::Downstream))
            .unwrap()[0];
        assert_eq!(caret.source(), CaretSource::FontAuthored);
        assert_eq!(caret.x(), 220.0);
        assert_eq!(caret.quality(), CaretQuality::Approximate);
        assert_eq!(
            layout.hit_test(220.0, 0.0).unwrap().source(),
            CaretSource::FontAuthored
        );
    }
}
