//! Bounded horizontal UAX #9 layout with hard LF/CRLF lines.
//! Unicode 16.0 BiDi levels/order are resolved on original text before controls
//! are hidden. Shaping retains logical order; visual runs are translated, never
//! reversed. No wrapping, vertical layout, overrides, or fallback is performed.
use crate::caret::{authored_carets, CaretSource};
use crate::wrapped::validate_paragraph_options;
use crate::{
    CaretAffinity, CaretQuality, FontEngine, FontFamily, FontOptions, GlyphRun, GlyphSource,
    ParagraphOptions, PositionedGlyph, ShapedText, TextAlignment, TextDirection, TextPosition,
    WrapMode,
};
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
        let line_height = engine
            .options()
            .line_height
            .unwrap_or(engine.options().font_size);
        Self::shape_internal(
            text,
            base,
            line_height,
            None,
            WrapMode::Grapheme,
            ParagraphOptions::default(),
            &mut |slice, direction| engine.shape_bidi_run_mapped(slice, direction),
        )
    }

    /// Shapes hard-line horizontal BiDi text using [`FontFamily`] fallback.
    pub fn shape_family(
        family: &FontFamily,
        text: &str,
        options: FontOptions<'_>,
        base: BidiBaseDirection,
    ) -> Result<Self, Error> {
        if options.text_direction == TextDirection::TopToBottom {
            return Err(unsupported("BiDi paragraph layout is horizontal only"));
        }
        family.shape_bidi_run_mapped("", options.clone(), TextDirection::LeftToRight)?;
        let line_height = options.line_height.unwrap_or(options.font_size);
        Self::shape_internal(
            text,
            base,
            line_height,
            None,
            WrapMode::Grapheme,
            ParagraphOptions::default(),
            &mut |slice, direction| family.shape_bidi_run_mapped(slice, options.clone(), direction),
        )
    }

    /// Shapes and soft-wraps horizontal BiDi text using a single [`FontEngine`].
    pub fn shape_wrapped(
        engine: &FontEngine<'_>,
        text: &str,
        base: BidiBaseDirection,
        max_inline_extent: f32,
        wrap_mode: WrapMode,
    ) -> Result<Self, Error> {
        Self::shape_wrapped_with_options(
            engine,
            text,
            base,
            max_inline_extent,
            wrap_mode,
            ParagraphOptions::default(),
        )
    }

    /// Shapes and soft-wraps horizontal BiDi text with [`ParagraphOptions`].
    pub fn shape_wrapped_with_options(
        engine: &FontEngine<'_>,
        text: &str,
        base: BidiBaseDirection,
        max_inline_extent: f32,
        wrap_mode: WrapMode,
        paragraph: ParagraphOptions,
    ) -> Result<Self, Error> {
        engine.shape_mapped("")?;
        if engine.options().text_direction == TextDirection::TopToBottom {
            return Err(unsupported("BiDi paragraph layout is horizontal only"));
        }
        let line_height = engine
            .options()
            .line_height
            .unwrap_or(engine.options().font_size);
        Self::shape_internal(
            text,
            base,
            line_height,
            Some(max_inline_extent),
            wrap_mode,
            paragraph,
            &mut |slice, direction| engine.shape_bidi_run_mapped(slice, direction),
        )
    }

    /// Shapes and soft-wraps horizontal BiDi text using [`FontFamily`] fallback.
    pub fn shape_family_wrapped(
        family: &FontFamily,
        text: &str,
        options: FontOptions<'_>,
        base: BidiBaseDirection,
        max_inline_extent: f32,
        wrap_mode: WrapMode,
    ) -> Result<Self, Error> {
        Self::shape_family_wrapped_with_options(
            family,
            text,
            options,
            base,
            max_inline_extent,
            wrap_mode,
            ParagraphOptions::default(),
        )
    }

    /// Shapes and soft-wraps horizontal BiDi text using [`FontFamily`] fallback
    /// and [`ParagraphOptions`].
    pub fn shape_family_wrapped_with_options(
        family: &FontFamily,
        text: &str,
        options: FontOptions<'_>,
        base: BidiBaseDirection,
        max_inline_extent: f32,
        wrap_mode: WrapMode,
        paragraph: ParagraphOptions,
    ) -> Result<Self, Error> {
        if options.text_direction == TextDirection::TopToBottom {
            return Err(unsupported("BiDi paragraph layout is horizontal only"));
        }
        family.shape_bidi_run_mapped("", options.clone(), TextDirection::LeftToRight)?;
        let line_height = options.line_height.unwrap_or(options.font_size);
        Self::shape_internal(
            text,
            base,
            line_height,
            Some(max_inline_extent),
            wrap_mode,
            paragraph,
            &mut |slice, direction| family.shape_bidi_run_mapped(slice, options.clone(), direction),
        )
    }

    fn shape_internal<F>(
        text: &str,
        base: BidiBaseDirection,
        line_height: f32,
        max_inline_extent: Option<f32>,
        wrap_mode: WrapMode,
        paragraph_options: ParagraphOptions,
        shape_run: &mut F,
    ) -> Result<Self, Error>
    where
        F: FnMut(&str, TextDirection) -> Result<ShapedText, Error>,
    {
        if !line_height.is_finite() || line_height <= 0.0 {
            return Err(invalid("line height must be positive and finite"));
        }
        if let Some(limit) = max_inline_extent {
            if !limit.is_finite() || limit <= 0.0 {
                return Err(invalid("max inline extent must be positive and finite"));
            }
            validate_paragraph_options(&paragraph_options, limit)?;
        }

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

        for (para_range, para_break) in ranges {
            let line_text = &text[para_range.clone()];
            let allow_tabs = paragraph_options.tab_width.is_some();
            if line_text.chars().any(|c| {
                (c.is_control() && !(allow_tabs && c == '\t'))
                    || matches!(c, '\u{2028}' | '\u{2029}' | '\u{202d}' | '\u{202e}')
            }) {
                return Err(unsupported(
                    "only LF/CRLF lines without tabs or directional overrides are supported",
                ));
            }

            let level = match base {
                BidiBaseDirection::Auto => None,
                BidiBaseDirection::LeftToRight => Some(Level::ltr()),
                BidiBaseDirection::RightToLeft => Some(Level::rtl()),
            };
            // Analysis must see controls; filtering them beforehand changes UAX #9.
            let bidi = BidiInfo::new(line_text, level);
            let base_level = if let Some(para) = bidi.paragraphs.first() {
                if bidi.paragraphs.len() != 1 {
                    return Err(unsupported("unexpected paragraph separator"));
                }
                para.level.number()
            } else {
                level.unwrap_or(Level::ltr()).number()
            };

            let mut boundaries = vec![0];
            boundaries.extend(line_text.grapheme_indices(true).map(|(i, g)| i + g.len()));

            if line_text.is_empty() {
                let line_index = lines.len();
                let baseline = line_index as f32 * line_height;
                if !baseline.is_finite() {
                    return Err(invalid("line baseline overflow"));
                }
                let x_offset = compute_bidi_line_x_offset(
                    max_inline_extent,
                    paragraph_options,
                    base_level,
                    paragraph_options.first_line_indent,
                    0.0,
                );
                for affinity in [CaretAffinity::Upstream, CaretAffinity::Downstream] {
                    carets.push(BidiCaretPosition {
                        position: TextPosition::new(para_range.start, affinity),
                        x: x_offset,
                        y: baseline,
                        line_index,
                        quality: CaretQuality::AdvanceBoundary,
                        source: CaretSource::Advance,
                    });
                }
                lines.push(BidiLine {
                    range: para_range,
                    line_break: para_break,
                    runs: Vec::new(),
                    base_level,
                    width: 0.0,
                    baseline,
                });
                continue;
            }

            let para = &bidi.paragraphs[0];
            let egcs: Vec<Range<usize>> = line_text
                .grapheme_indices(true)
                .map(|(i, g)| i..i + g.len())
                .collect();

            let mut visual_slices: Vec<(Range<usize>, Range<usize>, bool, f32)> = Vec::new();
            match max_inline_extent {
                None => {
                    visual_slices.push((0..line_text.len(), para_break, false, 0.0));
                }
                Some(limit) => {
                    let mut egc_idx = 0usize;
                    while egc_idx < egcs.len() {
                        let start_local = egcs[egc_idx].start;
                        let remaining = egcs.len() - egc_idx;
                        let indent = if egc_idx == 0 {
                            paragraph_options.first_line_indent
                        } else {
                            0.0
                        };
                        let line_limit = limit - indent;

                        let rem_width = measure_bidi_slice_width(
                            line_text,
                            &bidi,
                            para,
                            &boundaries,
                            start_local..line_text.len(),
                            paragraph_options.tab_width,
                            shape_run,
                        )?;
                        if rem_width <= line_limit {
                            visual_slices.push((
                                start_local..line_text.len(),
                                para_break.clone(),
                                false,
                                indent,
                            ));
                            break;
                        }

                        let mut best_count: Option<usize> = None;
                        let mut lo = 1usize;
                        let mut hi = remaining.saturating_sub(1);
                        while lo <= hi {
                            let mid = lo + (hi - lo) / 2;
                            let end_local = egcs[egc_idx + mid - 1].end;
                            let w = measure_bidi_slice_width(
                                line_text,
                                &bidi,
                                para,
                                &boundaries,
                                start_local..end_local,
                                paragraph_options.tab_width,
                                shape_run,
                            )?;
                            if w <= line_limit {
                                best_count = Some(mid);
                                lo = mid + 1;
                            } else {
                                hi = mid.saturating_sub(1);
                            }
                        }

                        let mut chosen_count = best_count.unwrap_or(1);
                        if wrap_mode == WrapMode::WordOrGrapheme && chosen_count < remaining {
                            if let Some(ws_count) = (1..=chosen_count).rev().find(|&count| {
                                let g = &line_text[egcs[egc_idx + count - 1].clone()];
                                g == " " || g == "\u{3000}"
                            }) {
                                if ws_count != chosen_count {
                                    chosen_count = ws_count;
                                }
                            }
                        }

                        let end_local = egcs[egc_idx + chosen_count - 1].end;
                        let is_last = egc_idx + chosen_count == egcs.len();
                        let (lbreak, is_soft) = if is_last {
                            (para_break.clone(), false)
                        } else {
                            let g = para_range.start + end_local;
                            (g..g, true)
                        };
                        visual_slices.push((start_local..end_local, lbreak, is_soft, indent));
                        egc_idx += chosen_count;
                    }
                }
            }

            for (vis_range, vis_break, is_soft_end, indent) in visual_slices {
                let line_index = lines.len();
                let baseline = line_index as f32 * line_height;
                if !baseline.is_finite() {
                    return Err(invalid("line baseline overflow"));
                }
                let is_soft_start = vis_range.start > 0;
                let (mut runs, mut line_carets, mut width) = shape_bidi_visual_line(
                    line_text,
                    para_range.start,
                    &bidi,
                    para,
                    &boundaries,
                    vis_range.clone(),
                    line_index,
                    baseline,
                    is_soft_start,
                    is_soft_end,
                    paragraph_options.tab_width,
                    shape_run,
                )?;

                if let Some(limit) = max_inline_extent {
                    let extra = (limit - indent - width).max(0.0);
                    if paragraph_options.alignment == TextAlignment::Justify
                        && is_soft_end
                        && extra > 0.0
                        && justify_bidi_line(&mut runs, &mut line_carets, extra)
                    {
                        width += extra;
                    }
                    let x_offset = compute_bidi_line_x_offset(
                        Some(limit),
                        paragraph_options,
                        base_level,
                        indent,
                        width,
                    );
                    if x_offset != 0.0 {
                        for run in &mut runs {
                            run.origin_x += x_offset;
                        }
                        for c in &mut line_carets {
                            c.x += x_offset;
                        }
                    }
                }

                carets.extend(line_carets);
                lines.push(BidiLine {
                    range: (para_range.start + vis_range.start)..(para_range.start + vis_range.end),
                    line_break: vis_break,
                    runs,
                    base_level,
                    width,
                    baseline,
                });
            }
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

#[allow(clippy::too_many_arguments)]
fn measure_bidi_slice_width<F>(
    line_text: &str,
    bidi: &BidiInfo<'_>,
    para: &unicode_bidi::ParagraphInfo,
    boundaries: &[usize],
    vis_range: Range<usize>,
    tab_width: Option<f32>,
    shape_run: &mut F,
) -> Result<f32, Error>
where
    F: FnMut(&str, TextDirection) -> Result<ShapedText, Error>,
{
    let (_, _, width) = shape_bidi_visual_line(
        line_text, 0, bidi, para, boundaries, vis_range, 0, 0.0, false, false, tab_width, shape_run,
    )?;
    Ok(width)
}

#[allow(clippy::too_many_arguments)]
fn shape_bidi_visual_line<F>(
    line_text: &str,
    para_global_start: usize,
    bidi: &BidiInfo<'_>,
    para: &unicode_bidi::ParagraphInfo,
    boundaries: &[usize],
    vis_range: Range<usize>,
    line_index: usize,
    baseline: f32,
    is_soft_start: bool,
    is_soft_end: bool,
    tab_width: Option<f32>,
    shape_run: &mut F,
) -> Result<(Vec<BidiVisualRun>, Vec<BidiCaretPosition>, f32), Error>
where
    F: FnMut(&str, TextDirection) -> Result<ShapedText, Error>,
{
    let (levels, visual) = bidi.visual_runs(para, vis_range.clone());
    #[cfg(feature = "layout")]
    {
        let slice_text = &line_text[vis_range.clone()];
        let slice_runs: Vec<Range<usize>> = visual
            .iter()
            .map(|r| (r.start - vis_range.start)..(r.end - vis_range.start))
            .collect();
        validate_joining_boundaries(slice_text, &slice_runs)?;
    }

    let mut runs = Vec::new();
    let mut carets = Vec::new();
    let mut width = 0.0f32;

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
        let run_text = &line_text[local.clone()];
        let (shaped, stops, advance) = if let Some(tw) = tab_width {
            if run_text.contains('\t') {
                shape_bidi_run_with_tabs(run_text, direction, tw, shape_run)?
            } else {
                let shaped = shape_run(run_text, direction)?;
                let (stops, advance) = run_stops(&shaped)?;
                (shaped, stops, advance)
            }
        } else {
            let shaped = shape_run(run_text, direction)?;
            let (stops, advance) = run_stops(&shaped)?;
            (shaped, stops, advance)
        };

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
            let global = para_global_start + local.start + offset;
            for affinity in [CaretAffinity::Upstream, CaretAffinity::Downstream] {
                let at_run_start = offset == 0;
                let at_run_end = offset == shaped.text().len();
                if (at_run_start
                    && affinity == CaretAffinity::Upstream
                    && (local.start != vis_range.start || is_soft_start))
                    || (at_run_end
                        && affinity == CaretAffinity::Downstream
                        && (local.end != vis_range.end || is_soft_end))
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
            range: para_global_start + local.start..para_global_start + local.end,
            level: run_level.number(),
            shaped,
            origin_x,
            origin_y: baseline,
        });
        width = next_width;
    }

    Ok((runs, carets, width))
}

fn shape_bidi_run_with_tabs<F>(
    run_text: &str,
    direction: TextDirection,
    tab_width: f32,
    shape_run: &mut F,
) -> Result<(ShapedText, Vec<RunStop>, f32), Error>
where
    F: FnMut(&str, TextDirection) -> Result<ShapedText, Error>,
{
    let sign = if direction == TextDirection::RightToLeft {
        -1.0f32
    } else {
        1.0f32
    };
    let mut glyphs: Vec<PositionedGlyph> = Vec::new();
    let mut sources: Vec<GlyphSource> = Vec::new();
    let mut stops: Vec<RunStop> =
        vec![(0, 0.0, CaretQuality::AdvanceBoundary, CaretSource::Advance)];
    let mut pen = 0.0f32;
    let mut seg_start = 0usize;
    let mut run_index = 0usize;

    for (idx, ch) in run_text.char_indices() {
        if ch == '\t' {
            if seg_start < idx {
                let seg_shaped = shape_run(&run_text[seg_start..idx], direction)?;
                let (seg_stops, seg_advance) = run_stops(&seg_shaped)?;
                let dx = pen * sign;
                for mut g in seg_shaped.glyph_run().glyphs.iter().cloned() {
                    g.x += dx;
                    glyphs.push(g);
                }
                for s in seg_shaped.glyph_sources() {
                    let r = s.byte_range();
                    sources.push(GlyphSource::new(
                        (seg_start + r.start)..(seg_start + r.end),
                        s.face_index(),
                        run_index,
                    ));
                }
                for (off, inline, quality, src) in seg_stops.into_iter().skip(1) {
                    stops.push((seg_start + off, pen + inline, quality, src));
                }
                pen += seg_advance;
                run_index += 1;
            }
            let next_tab = ((pen / tab_width).floor() + 1.0) * tab_width;
            pen = next_tab.max(pen);
            stops.push((
                idx + 1,
                pen,
                CaretQuality::AdvanceBoundary,
                CaretSource::Advance,
            ));
            seg_start = idx + 1;
        }
    }
    if seg_start < run_text.len() {
        let seg_shaped = shape_run(&run_text[seg_start..], direction)?;
        let (seg_stops, seg_advance) = run_stops(&seg_shaped)?;
        let dx = pen * sign;
        for mut g in seg_shaped.glyph_run().glyphs.iter().cloned() {
            g.x += dx;
            glyphs.push(g);
        }
        for s in seg_shaped.glyph_sources() {
            let r = s.byte_range();
            sources.push(GlyphSource::new(
                (seg_start + r.start)..(seg_start + r.end),
                s.face_index(),
                run_index,
            ));
        }
        for (off, inline, quality, src) in seg_stops.into_iter().skip(1) {
            stops.push((seg_start + off, pen + inline, quality, src));
        }
        pen += seg_advance;
    }

    let shaped = ShapedText::new(run_text, GlyphRun::new(glyphs), sources, direction);
    Ok((shaped, stops, pen))
}

fn compute_bidi_line_x_offset(
    max_inline_extent: Option<f32>,
    options: ParagraphOptions,
    base_level: u8,
    indent: f32,
    width: f32,
) -> f32 {
    let Some(limit) = max_inline_extent else {
        return 0.0;
    };
    let is_rtl = base_level % 2 == 1;
    let extra = (limit - indent - width).max(0.0);
    match options.alignment {
        TextAlignment::Start => {
            if is_rtl {
                extra
            } else {
                indent
            }
        }
        TextAlignment::Center => {
            if is_rtl {
                extra / 2.0
            } else {
                indent + extra / 2.0
            }
        }
        TextAlignment::End => {
            if is_rtl {
                0.0
            } else {
                indent + extra
            }
        }
        TextAlignment::Justify => {
            if is_rtl {
                extra
            } else {
                indent
            }
        }
    }
}

fn justify_bidi_line(
    runs: &mut [BidiVisualRun],
    carets: &mut [BidiCaretPosition],
    extra: f32,
) -> bool {
    let mut space_positions: Vec<(usize, usize)> = Vec::new();
    let total_runs = runs.len();
    for (run_idx, run) in runs.iter().enumerate() {
        let text = run.shaped.text();
        let egcs: Vec<(usize, &str)> = text.grapheme_indices(true).collect();
        for (idx, (start, g)) in egcs.iter().enumerate() {
            let is_last_of_line = run_idx + 1 == total_runs && idx + 1 == egcs.len();
            if !is_last_of_line && (g == &" " || g == &"\u{3000}") {
                space_positions.push((run_idx, *start));
            }
        }
    }
    if space_positions.is_empty() {
        return false;
    }
    let gap = extra / space_positions.len() as f32;
    let mut cumulative_shifts = Vec::with_capacity(runs.len());
    let mut spaces_before = 0usize;
    for (run_idx, run) in runs.iter_mut().enumerate() {
        let is_rtl = run.level % 2 == 1;
        let local_spaces: Vec<usize> = space_positions
            .iter()
            .filter(|&&(ri, _)| ri == run_idx)
            .map(|&(_, off)| off)
            .collect();
        let run_shift = spaces_before as f32 * gap;
        run.origin_x += run_shift;
        if is_rtl {
            // In RTL runs, visually right-to-left means earlier logical bytes are to the right.
            // Expanding a space pushes preceding (visually right) glyphs right by +gap per later space,
            // or equivalently shifts origin_x by local_spaces.len() * gap and pushes later glyphs left.
            let local_total = local_spaces.len() as f32 * gap;
            run.origin_x += local_total;
            let sources: Vec<Range<usize>> = run
                .shaped
                .glyph_sources()
                .iter()
                .map(|s| s.byte_range())
                .collect();
            for (glyph, range) in run.shaped.glyph_run_mut().glyphs.iter_mut().zip(sources) {
                let spaces_after_in_run =
                    local_spaces.iter().filter(|&&sp| sp >= range.end).count();
                glyph.x -= spaces_after_in_run as f32 * gap;
            }
        } else {
            let sources: Vec<Range<usize>> = run
                .shaped
                .glyph_sources()
                .iter()
                .map(|s| s.byte_range())
                .collect();
            for (glyph, range) in run.shaped.glyph_run_mut().glyphs.iter_mut().zip(sources) {
                let spaces_before_in_run =
                    local_spaces.iter().filter(|&&sp| range.start > sp).count();
                glyph.x += spaces_before_in_run as f32 * gap;
            }
        }
        cumulative_shifts.push((run.range.clone(), is_rtl, run_shift, local_spaces.clone()));
        spaces_before += local_spaces.len();
    }

    for caret in carets.iter_mut() {
        let offset = caret.position.byte_offset();
        if let Some((range, is_rtl, run_shift, local_spaces)) =
            cumulative_shifts.iter().find(|(r, _, _, _)| {
                if caret.position.affinity() == CaretAffinity::Upstream {
                    offset > r.start && offset <= r.end
                } else {
                    offset >= r.start && offset < r.end
                }
            })
        {
            let local_off = offset - range.start;
            if *is_rtl {
                let local_total = local_spaces.len() as f32 * gap;
                let spaces_after = local_spaces.iter().filter(|&&sp| local_off > sp).count();
                caret.x += run_shift + local_total - spaces_after as f32 * gap;
            } else {
                let spaces_before_in_run =
                    local_spaces.iter().filter(|&&sp| local_off > sp).count();
                caret.x += run_shift + spaces_before_in_run as f32 * gap;
            }
        } else if offset == runs.last().map(|r| r.range.end).unwrap_or(0) {
            caret.x += spaces_before as f32 * gap;
        }
    }
    true
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
    #[cfg(feature = "layout")]
    #[test]
    fn paragraph_keeps_format_two_point_carets_in_global_run_coordinates() {
        let file =
            FontFile::from_buffer(include_bytes!("../tests/fixtures/gdef-carets.ttc")).unwrap();
        let face = file.face(1).unwrap();
        let engine = face.engine().with_font_size(1000.0);
        let layout =
            BidiParagraphLayout::shape(&engine, "fifl", BidiBaseDirection::LeftToRight).unwrap();
        for (byte, x) in [(1, 280.0), (3, 1730.0)] {
            let caret = layout
                .caret_positions(position(byte, CaretAffinity::Downstream))
                .unwrap()[0];
            assert_eq!(caret.x(), x);
            assert_eq!(caret.source(), CaretSource::FontAuthored);
            let hit = layout.hit_test(x, 0.0).unwrap();
            assert_eq!(hit.text_position().byte_offset(), byte);
            assert_eq!(hit.source(), CaretSource::FontAuthored);
        }
    }
}
