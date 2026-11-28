//! Bounded soft-wrap line layout built on mapped shaping and caret geometry.
//!
//! Each hard paragraph (split by LF or CRLF) is greedily wrapped against a
//! maximum inline extent and re-shaped per visual line so ligatures, kerning,
//! and contextual forms remain valid on every visual line.
//!
//! This is intentionally explicit and bounded: it does not claim full Unicode
//! UAX #14 line breaking, hyphenation, justification, or paragraph BiDi.
use crate::lines::{select_nearest_line_index, split_hard_lines};
use crate::{
    CaretAffinity, CaretLayout, CaretPosition, CaretQuality, CaretSource, CompositeFont,
    FontEngine, FontFamily, FontOptions, GlyphRun, GlyphSource, LineCaretPosition, ShapedText,
    TextDirection, TextPosition,
};
use std::io::{Error, ErrorKind};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// Alignment of wrapped visual lines within `max_inline_extent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlignment {
    #[default]
    Start,
    Center,
    End,
    Justify,
}

/// Paragraph-level layout options for alignment, first-line indent, and tab stops.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParagraphOptions {
    pub alignment: TextAlignment,
    pub first_line_indent: f32,
    pub tab_width: Option<f32>,
}

impl Default for ParagraphOptions {
    fn default() -> Self {
        Self {
            alignment: TextAlignment::Start,
            first_line_indent: 0.0,
            tab_width: None,
        }
    }
}

pub(crate) fn validate_paragraph_options(
    options: &ParagraphOptions,
    max_inline_extent: f32,
) -> Result<(), Error> {
    if !options.first_line_indent.is_finite()
        || options.first_line_indent < 0.0
        || options.first_line_indent >= max_inline_extent
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "first_line_indent must be finite, non-negative, and smaller than max_inline_extent",
        ));
    }
    if let Some(tab_width) = options.tab_width {
        if !tab_width.is_finite() || tab_width <= 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "tab_width must be positive and finite",
            ));
        }
    }
    Ok(())
}

/// Strategy for selecting soft-wrap break points when a line exceeds the
/// maximum inline extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapMode {
    /// Prefer breaking after ASCII space (`' '`) or ideographic space (`'\u{3000}'`)
    /// when a break fits; otherwise fall back to extended grapheme cluster boundaries.
    WordOrGrapheme,
    /// Break at any Unicode 17 extended grapheme cluster boundary.
    Grapheme,
    /// Apply Japanese Kinsoku line-breaking rules (push-out of line-start/line-end
    /// prohibited characters and inseparable pairs) on top of EGC boundaries.
    JapaneseKinsoku,
    /// Like [`WrapMode::JapaneseKinsoku`], allowing a single trailing Japanese
    /// punctuation mark (`、。，．`) to hang beyond `max_inline_extent`.
    JapaneseKinsokuHanging,
}

/// How a visual line terminates in [`WrappedLineLayout`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineBreakKind {
    /// Terminated by an explicit LF or CRLF in the original text.
    Hard,
    /// Wrapped because the remaining paragraph exceeded `max_inline_extent`.
    Soft,
    /// Final visual line of the input text.
    EndOfText,
}

/// One shaped visual line in a [`WrappedLineLayout`].
///
/// To render, draw `shaped().glyph_run()` translated by `(origin_x(), origin_y())`.
/// Source ranges inside `shaped()` are line-local; add `content_range().start`.
#[derive(Debug, Clone)]
pub struct WrappedLine {
    content: Range<usize>,
    line_break: Range<usize>,
    break_kind: LineBreakKind,
    shaped: ShapedText,
    carets: CaretLayout,
    origin_x: f32,
    origin_y: f32,
    inline_advance: f32,
}

impl WrappedLine {
    /// Original UTF-8 byte range of this visual line, excluding hard-break bytes.
    pub fn content_range(&self) -> Range<usize> {
        self.content.clone()
    }

    /// Original LF or CRLF byte range when `break_kind() == LineBreakKind::Hard`;
    /// empty for soft-wrapped and final lines.
    pub fn break_range(&self) -> Range<usize> {
        self.line_break.clone()
    }

    /// Whether this visual line ended with a hard break, soft wrap, or end of text.
    pub fn break_kind(&self) -> LineBreakKind {
        self.break_kind
    }

    /// Line-local shaped text for this visual line.
    pub fn shaped(&self) -> &ShapedText {
        &self.shaped
    }

    pub fn origin_x(&self) -> f32 {
        self.origin_x
    }

    pub fn origin_y(&self) -> f32 {
        self.origin_y
    }

    /// Non-negative inline advance of this visual line's shaped glyphs.
    pub fn inline_advance(&self) -> f32 {
        self.inline_advance
    }
}

/// Multi-line layout with explicit LF/CRLF hard breaks and bounded soft wrapping.
///
/// Horizontal lines start at `(0, line_index * line_height)`; TTB columns start
/// at `(-line_index * line_height, 0)`. At soft-wrap boundaries, the shared byte
/// offset resolves to the end of the preceding visual line with
/// [`CaretAffinity::Upstream`] and to the start of the following visual line with
/// [`CaretAffinity::Downstream`].
#[derive(Debug, Clone)]
pub struct WrappedLineLayout {
    text: String,
    lines: Vec<WrappedLine>,
    direction: TextDirection,
    line_height: f32,
    max_inline_extent: f32,
    wrap_mode: WrapMode,
}

struct PendingVisualLine {
    content: Range<usize>,
    line_break: Range<usize>,
    break_kind: LineBreakKind,
    shaped: ShapedText,
    carets: CaretLayout,
    inline_advance: f32,
    indent: f32,
}

impl WrappedLineLayout {
    /// Shapes and wraps `text` using a single [`FontEngine`].
    pub fn shape(
        engine: &FontEngine<'_>,
        text: &str,
        max_inline_extent: f32,
        wrap_mode: WrapMode,
    ) -> Result<Self, Error> {
        Self::shape_with_paragraph(
            engine,
            text,
            max_inline_extent,
            wrap_mode,
            ParagraphOptions::default(),
        )
    }

    /// Shapes and wraps `text` using a single [`FontEngine`] and [`ParagraphOptions`].
    pub fn shape_with_paragraph(
        engine: &FontEngine<'_>,
        text: &str,
        max_inline_extent: f32,
        wrap_mode: WrapMode,
        paragraph: ParagraphOptions,
    ) -> Result<Self, Error> {
        let options = engine.options();
        let line_height = options.line_height.unwrap_or(options.font_size);
        let direction = options.text_direction;
        Self::shape_with_options(
            text,
            direction,
            line_height,
            max_inline_extent,
            wrap_mode,
            paragraph,
            |slice| engine.shape_mapped(slice),
        )
    }

    /// Shapes and wraps `text` using [`FontFamily`] fallback.
    pub fn shape_family(
        family: &FontFamily,
        text: &str,
        options: FontOptions<'_>,
        max_inline_extent: f32,
        wrap_mode: WrapMode,
    ) -> Result<Self, Error> {
        Self::shape_family_with_paragraph(
            family,
            text,
            options,
            max_inline_extent,
            wrap_mode,
            ParagraphOptions::default(),
        )
    }

    /// Shapes and wraps `text` using [`FontFamily`] fallback and [`ParagraphOptions`].
    pub fn shape_family_with_paragraph(
        family: &FontFamily,
        text: &str,
        options: FontOptions<'_>,
        max_inline_extent: f32,
        wrap_mode: WrapMode,
        paragraph: ParagraphOptions,
    ) -> Result<Self, Error> {
        let line_height = options.line_height.unwrap_or(options.font_size);
        let direction = options.text_direction;
        Self::shape_with_options(
            text,
            direction,
            line_height,
            max_inline_extent,
            wrap_mode,
            paragraph,
            |slice| family.shape_mapped(slice, options.clone()),
        )
    }

    /// Shapes and wraps horizontal LTR `text` using [`CompositeFont`].
    pub fn shape_composite_ltr(
        composite: &CompositeFont<'_>,
        text: &str,
        max_inline_extent: f32,
        wrap_mode: WrapMode,
    ) -> Result<Self, Error> {
        let options = composite.options();
        let line_height = options.line_height.unwrap_or(options.font_size);
        Self::shape_with_options(
            text,
            TextDirection::LeftToRight,
            line_height,
            max_inline_extent,
            wrap_mode,
            ParagraphOptions::default(),
            |slice| composite.shape_mapped_ltr(slice),
        )
    }

    fn shape_with_options<F>(
        text: &str,
        direction: TextDirection,
        line_height: f32,
        max_inline_extent: f32,
        wrap_mode: WrapMode,
        paragraph: ParagraphOptions,
        mut shape_line: F,
    ) -> Result<Self, Error>
    where
        F: FnMut(&str) -> Result<ShapedText, Error>,
    {
        if !max_inline_extent.is_finite() || max_inline_extent <= 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "max inline extent must be positive and finite",
            ));
        }
        validate_paragraph_options(&paragraph, max_inline_extent)?;
        // Validate font and options even when no visible glyph is requested.
        shape_line("")?;
        if !line_height.is_finite() || line_height <= 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "line height must be positive and finite",
            ));
        }

        let hard_paragraphs = split_hard_lines(text);
        let last_para_index = hard_paragraphs.len().saturating_sub(1);
        let mut pending = Vec::with_capacity(hard_paragraphs.len());

        for (para_index, (para_content, para_break)) in hard_paragraphs.into_iter().enumerate() {
            let terminal_kind = if !para_break.is_empty() {
                LineBreakKind::Hard
            } else if para_index == last_para_index {
                LineBreakKind::EndOfText
            } else {
                LineBreakKind::Hard
            };

            let para_text = &text[para_content.clone()];
            let (para_shaped, para_carets, para_advance) =
                shape_and_measure(&mut shape_line, para_text, direction, paragraph.tab_width)?;
            let first_limit = max_inline_extent - paragraph.first_line_indent;

            if para_text.is_empty() || para_advance <= first_limit {
                pending.push(PendingVisualLine {
                    content: para_content,
                    line_break: para_break,
                    break_kind: terminal_kind,
                    shaped: para_shaped,
                    carets: para_carets,
                    inline_advance: para_advance,
                    indent: paragraph.first_line_indent,
                });
                continue;
            }

            let egcs: Vec<Range<usize>> = para_shaped.grapheme_ranges().to_vec();
            let mut egc_idx = 0usize;

            while egc_idx < egcs.len() {
                let start_local = egcs[egc_idx].start;
                let remaining_egcs = egcs.len() - egc_idx;
                let indent = if egc_idx == 0 {
                    paragraph.first_line_indent
                } else {
                    0.0
                };
                let line_limit = max_inline_extent - indent;

                if egc_idx > 0 {
                    let rem_text = &para_text[start_local..];
                    let (rem_shaped, rem_carets, rem_advance) = shape_and_measure(
                        &mut shape_line,
                        rem_text,
                        direction,
                        paragraph.tab_width,
                    )?;
                    if rem_advance <= line_limit {
                        pending.push(PendingVisualLine {
                            content: (para_content.start + start_local)..para_content.end,
                            line_break: para_break.clone(),
                            break_kind: terminal_kind,
                            shaped: rem_shaped,
                            carets: rem_carets,
                            inline_advance: rem_advance,
                            indent,
                        });
                        break;
                    }
                }

                // Binary search the largest EGC count in [1, remaining_egcs - 1]
                // whose shaped inline advance fits within line_limit.
                let mut best: Option<(usize, ShapedText, CaretLayout, f32)> = None;
                let mut lo = 1usize;
                let mut hi = remaining_egcs.saturating_sub(1);
                while lo <= hi {
                    let mid = lo + (hi - lo) / 2;
                    let end_local = egcs[egc_idx + mid - 1].end;
                    let slice = &para_text[start_local..end_local];
                    let (shaped, carets, advance) =
                        shape_and_measure(&mut shape_line, slice, direction, paragraph.tab_width)?;
                    if advance <= line_limit {
                        best = Some((mid, shaped, carets, advance));
                        lo = mid + 1;
                    } else {
                        hi = mid.saturating_sub(1);
                    }
                }

                let (mut chosen_count, mut chosen_shaped, mut chosen_carets, mut chosen_advance) =
                    if let Some(found) = best {
                        found
                    } else {
                        // Single EGC exceeds line_limit: emit 1 EGC to guarantee progress.
                        let end_local = egcs[egc_idx].end;
                        let (shaped, carets, advance) = shape_and_measure(
                            &mut shape_line,
                            &para_text[start_local..end_local],
                            direction,
                            paragraph.tab_width,
                        )?;
                        (1, shaped, carets, advance)
                    };

                if chosen_count < remaining_egcs {
                    let adjusted_count = match wrap_mode {
                        WrapMode::WordOrGrapheme => (1..=chosen_count)
                            .rev()
                            .find(|&count| {
                                is_wrap_space(&para_text[egcs[egc_idx + count - 1].clone()])
                            })
                            .unwrap_or(chosen_count),
                        WrapMode::Grapheme => chosen_count,
                        WrapMode::JapaneseKinsoku => resolve_kinsoku_break_count(
                            para_text,
                            &egcs[egc_idx..],
                            chosen_count,
                            false,
                        ),
                        WrapMode::JapaneseKinsokuHanging => resolve_kinsoku_break_count(
                            para_text,
                            &egcs[egc_idx..],
                            chosen_count,
                            true,
                        ),
                    };
                    if adjusted_count != chosen_count {
                        let end_local = egcs[egc_idx + adjusted_count - 1].end;
                        let (shaped, carets, advance) = shape_and_measure(
                            &mut shape_line,
                            &para_text[start_local..end_local],
                            direction,
                            paragraph.tab_width,
                        )?;
                        let allowed = matches!(
                            wrap_mode,
                            WrapMode::JapaneseKinsoku | WrapMode::JapaneseKinsokuHanging
                        ) || advance <= line_limit;
                        if allowed {
                            chosen_count = adjusted_count;
                            chosen_shaped = shaped;
                            chosen_carets = carets;
                            chosen_advance = advance;
                        }
                    }
                }

                let end_local = egcs[egc_idx + chosen_count - 1].end;
                let global_start = para_content.start + start_local;
                let global_end = para_content.start + end_local;
                let is_para_end = egc_idx + chosen_count == egcs.len();

                pending.push(PendingVisualLine {
                    content: global_start..global_end,
                    line_break: if is_para_end {
                        para_break.clone()
                    } else {
                        global_end..global_end
                    },
                    break_kind: if is_para_end {
                        terminal_kind
                    } else {
                        LineBreakKind::Soft
                    },
                    shaped: chosen_shaped,
                    carets: chosen_carets,
                    inline_advance: chosen_advance,
                    indent,
                });

                egc_idx += chosen_count;
            }
        }

        let mut lines = Vec::with_capacity(pending.len());
        for (index, mut item) in pending.into_iter().enumerate() {
            let distance = index as f32 * line_height;
            if !distance.is_finite() {
                return Err(Error::new(ErrorKind::InvalidInput, "line origin overflow"));
            }
            let extra = (max_inline_extent - item.indent - item.inline_advance).max(0.0);
            let align_offset = match paragraph.alignment {
                TextAlignment::Start => item.indent,
                TextAlignment::Center => item.indent + extra / 2.0,
                TextAlignment::End => item.indent + extra,
                TextAlignment::Justify => {
                    if item.break_kind == LineBreakKind::Soft && extra > 0.0 {
                        let applied =
                            justify_line(&mut item.shaped, &mut item.carets, direction, extra);
                        if applied {
                            item.inline_advance += extra;
                        }
                    }
                    item.indent
                }
            };
            let (origin_x, origin_y) = match direction {
                TextDirection::TopToBottom => (-distance, align_offset),
                TextDirection::RightToLeft => (-align_offset, distance),
                TextDirection::LeftToRight => (align_offset, distance),
            };
            if item
                .shaped
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
            lines.push(WrappedLine {
                content: item.content,
                line_break: item.line_break,
                break_kind: item.break_kind,
                shaped: item.shaped,
                carets: item.carets,
                origin_x,
                origin_y,
                inline_advance: item.inline_advance,
            });
        }

        Ok(Self {
            text: text.to_owned(),
            lines,
            direction,
            line_height,
            max_inline_extent,
            wrap_mode,
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn lines(&self) -> &[WrappedLine] {
        &self.lines
    }

    pub fn direction(&self) -> TextDirection {
        self.direction
    }

    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    pub fn max_inline_extent(&self) -> f32 {
        self.max_inline_extent
    }

    pub fn wrap_mode(&self) -> WrapMode {
        self.wrap_mode
    }

    /// Returns all visual caret positions for a global UTF-8 EGC boundary.
    ///
    /// At a soft-wrap boundary, returns two positions: the end of the preceding
    /// line ([`CaretAffinity::Upstream`]) followed by the start of the next line
    /// ([`CaretAffinity::Downstream`]).
    pub fn visual_carets(&self, byte_offset: usize) -> Result<Vec<LineCaretPosition>, Error> {
        let matching: Vec<(usize, &WrappedLine)> = self
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| {
                byte_offset >= line.content.start && byte_offset <= line.content.end
            })
            .collect();
        if matching.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "not an available line-content boundary",
            ));
        }
        let mut out = Vec::with_capacity(matching.len());
        for (pos_idx, (line_index, line)) in matching.iter().enumerate() {
            let affinity = if matching.len() > 1 {
                if pos_idx == 0 {
                    CaretAffinity::Upstream
                } else {
                    CaretAffinity::Downstream
                }
            } else {
                CaretAffinity::Downstream
            };
            let local = TextPosition::new(byte_offset - line.content.start, affinity);
            for caret in line.carets.caret_positions(local)? {
                out.push(self.global_caret(*line_index, caret, affinity)?);
            }
        }
        Ok(out)
    }

    /// Resolves a global UTF-8 EGC boundary using its [`CaretAffinity`].
    ///
    /// At a soft-wrap boundary, [`CaretAffinity::Upstream`] selects the end of
    /// the preceding visual line and [`CaretAffinity::Downstream`] selects the
    /// start of the continuation line.
    pub fn caret_positions(&self, position: TextPosition) -> Result<Vec<LineCaretPosition>, Error> {
        let offset = position.byte_offset();
        let matching: Vec<(usize, &WrappedLine)> = self
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| offset >= line.content.start && offset <= line.content.end)
            .collect();
        if matching.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "not an available line-content boundary",
            ));
        }
        let (line_index, line) = if matching.len() > 1 {
            match position.affinity() {
                CaretAffinity::Upstream => matching[0],
                CaretAffinity::Downstream => *matching.last().unwrap(),
            }
        } else {
            matching[0]
        };
        let local = TextPosition::new(offset - line.content.start, position.affinity());
        line.carets
            .caret_positions(local)?
            .into_iter()
            .map(|caret| self.global_caret(line_index, caret, position.affinity()))
            .collect()
    }

    /// Picks the nearest visual line baseline on the cross axis, then the
    /// nearest inline caret stop on that line.
    ///
    /// At a soft-wrap boundary, the returned affinity is set to
    /// [`CaretAffinity::Upstream`] for the end of a wrapped line and
    /// [`CaretAffinity::Downstream`] for the start of a continuation line so
    /// [`Self::caret_positions`] round-trips to the same visual line.
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
        let local = if vertical {
            line.carets.hit_test(0.0, y - line.origin_y)?
        } else {
            line.carets.hit_test(x - line.origin_x, 0.0)?
        };
        let local_byte = local.text_position().byte_offset();
        let affinity = if local_byte == line.content.len() && line.break_kind == LineBreakKind::Soft
        {
            CaretAffinity::Upstream
        } else if local_byte == 0
            && best_index > 0
            && self.lines[best_index - 1].break_kind == LineBreakKind::Soft
        {
            CaretAffinity::Downstream
        } else {
            local.text_position().affinity()
        };
        self.global_caret(best_index, local, affinity)
    }

    fn global_caret(
        &self,
        index: usize,
        caret: CaretPosition,
        affinity: CaretAffinity,
    ) -> Result<LineCaretPosition, Error> {
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
                affinity,
            ),
            x,
            y,
            caret.quality(),
            caret.source(),
            index,
        ))
    }
}

fn shape_and_measure<F>(
    shape_line: &mut F,
    slice: &str,
    direction: TextDirection,
    tab_width: Option<f32>,
) -> Result<(ShapedText, CaretLayout, f32), Error>
where
    F: FnMut(&str) -> Result<ShapedText, Error>,
{
    if let Some(tw) = tab_width {
        if slice.contains('\t') {
            return shape_and_measure_with_tabs(shape_line, slice, direction, tw);
        }
    }
    let shaped = shape_line(slice)?;
    let carets = CaretLayout::new(&shaped)?;
    let advance = caret_inline_advance(&carets, direction);
    Ok((shaped, carets, advance))
}

fn shape_and_measure_with_tabs<F>(
    shape_line: &mut F,
    slice: &str,
    direction: TextDirection,
    tab_width: f32,
) -> Result<(ShapedText, CaretLayout, f32), Error>
where
    F: FnMut(&str) -> Result<ShapedText, Error>,
{
    let vertical = direction == TextDirection::TopToBottom;
    let sign = if direction == TextDirection::RightToLeft {
        -1.0f32
    } else {
        1.0f32
    };
    let mut glyphs = Vec::new();
    let mut sources = Vec::new();
    let mut stops = vec![CaretPosition::new(
        TextPosition::new(0, CaretAffinity::Downstream),
        0.0,
        0.0,
        CaretQuality::AdvanceBoundary,
        CaretSource::Advance,
    )];
    let mut pen = 0.0f32;
    let mut seg_start = 0usize;
    let mut run_index = 0usize;

    for (idx, ch) in slice.char_indices() {
        if ch == '\t' {
            if seg_start < idx {
                append_tab_segment(
                    shape_line,
                    &slice[seg_start..idx],
                    seg_start,
                    direction,
                    vertical,
                    sign,
                    &mut pen,
                    &mut glyphs,
                    &mut sources,
                    &mut stops,
                    &mut run_index,
                )?;
            }
            let next_tab = ((pen / tab_width).floor() + 1.0) * tab_width;
            pen = next_tab.max(pen);
            let (cx, cy) = if vertical {
                (0.0, pen)
            } else {
                (pen * sign, 0.0)
            };
            stops.push(CaretPosition::new(
                TextPosition::new(idx + 1, CaretAffinity::Downstream),
                cx,
                cy,
                CaretQuality::AdvanceBoundary,
                CaretSource::Advance,
            ));
            seg_start = idx + 1;
        }
    }
    if seg_start < slice.len() {
        append_tab_segment(
            shape_line,
            &slice[seg_start..],
            seg_start,
            direction,
            vertical,
            sign,
            &mut pen,
            &mut glyphs,
            &mut sources,
            &mut stops,
            &mut run_index,
        )?;
    }
    let shaped = ShapedText::new(slice, GlyphRun::new(glyphs), sources, direction);
    let carets = CaretLayout::from_stops(stops, vertical);
    Ok((shaped, carets, pen.max(0.0)))
}

#[allow(clippy::too_many_arguments)]
fn append_tab_segment<F>(
    shape_line: &mut F,
    seg: &str,
    seg_offset: usize,
    direction: TextDirection,
    vertical: bool,
    sign: f32,
    pen: &mut f32,
    glyphs: &mut Vec<crate::PositionedGlyph>,
    sources: &mut Vec<GlyphSource>,
    stops: &mut Vec<CaretPosition>,
    run_index: &mut usize,
) -> Result<(), Error>
where
    F: FnMut(&str) -> Result<ShapedText, Error>,
{
    let seg_shaped = shape_line(seg)?;
    let seg_carets = CaretLayout::new(&seg_shaped)?;
    let seg_advance = caret_inline_advance(&seg_carets, direction);
    let (dx, dy) = if vertical {
        (0.0, *pen)
    } else {
        (*pen * sign, 0.0)
    };
    for mut g in seg_shaped.glyph_run().glyphs.iter().cloned() {
        g.x += dx;
        g.y += dy;
        glyphs.push(g);
    }
    for s in seg_shaped.glyph_sources() {
        let r = s.byte_range();
        sources.push(GlyphSource::new(
            (seg_offset + r.start)..(seg_offset + r.end),
            s.face_index(),
            *run_index,
        ));
    }
    for stop in seg_carets.stops().iter().skip(1) {
        stops.push(CaretPosition::new(
            TextPosition::new(
                seg_offset + stop.text_position().byte_offset(),
                stop.text_position().affinity(),
            ),
            stop.x() + dx,
            stop.y() + dy,
            stop.quality(),
            stop.source(),
        ));
    }
    *pen += seg_advance;
    *run_index += 1;
    Ok(())
}

fn justify_line(
    shaped: &mut ShapedText,
    carets: &mut CaretLayout,
    direction: TextDirection,
    extra: f32,
) -> bool {
    let text = shaped.text().to_owned();
    let mut space_ends: Vec<usize> = Vec::new();
    let egcs: Vec<(usize, &str)> = text.grapheme_indices(true).collect();
    for (idx, (start, g)) in egcs.iter().enumerate() {
        if idx + 1 < egcs.len() && is_wrap_space(g) {
            space_ends.push(*start + g.len());
        }
    }
    if space_ends.is_empty() {
        return false;
    }
    let gap = extra / space_ends.len() as f32;
    let vertical = direction == TextDirection::TopToBottom;
    let sign = if direction == TextDirection::RightToLeft {
        -1.0f32
    } else {
        1.0f32
    };

    let sources: Vec<Range<usize>> = shaped
        .glyph_sources()
        .iter()
        .map(|s| s.byte_range())
        .collect();
    for (glyph, range) in shaped.glyph_run_mut().glyphs.iter_mut().zip(sources) {
        let count = space_ends.iter().filter(|&&end| range.start >= end).count();
        if count > 0 {
            let shift = gap * count as f32 * sign;
            if vertical {
                glyph.y += shift;
            } else {
                glyph.x += shift;
            }
        }
    }
    for stop in carets.stops_mut() {
        let offset = stop.text_position().byte_offset();
        let count = space_ends.iter().filter(|&&end| offset >= end).count();
        if count > 0 {
            stop.shift_inline(gap * count as f32 * sign, vertical);
        }
    }
    true
}

fn caret_inline_advance(carets: &CaretLayout, direction: TextDirection) -> f32 {
    let Some(last) = carets.stops().last() else {
        return 0.0;
    };
    match direction {
        TextDirection::LeftToRight => last.x().max(0.0),
        TextDirection::RightToLeft => (-last.x()).max(0.0),
        TextDirection::TopToBottom => last.y().max(0.0),
    }
}

fn is_wrap_space(egc: &str) -> bool {
    egc == " " || egc == "\u{3000}"
}

fn single_char(egc: &str) -> Option<char> {
    let mut chars = egc.chars();
    let ch = chars.next()?;
    if chars.next().is_none() {
        Some(ch)
    } else {
        None
    }
}

pub(crate) fn is_line_start_prohibited(egc: &str) -> bool {
    let Some(ch) = single_char(egc) else {
        return false;
    };
    matches!(
        ch,
        '、' | '。'
            | '，'
            | '．'
            | ','
            | '.'
            | '：'
            | '；'
            | ':'
            | ';'
            | '！'
            | '？'
            | '!'
            | '?'
            | '‼'
            | '⁇'
            | '⁈'
            | '⁉'
            | '）'
            | '］'
            | '｝'
            | '」'
            | '』'
            | '】'
            | '〕'
            | '〉'
            | '》'
            | '”'
            | '’'
            | ')'
            | ']'
            | '}'
            | '"'
            | '\''
            | '»'
            | '›'
            | '・'
            | '･'
            | 'ー'
            | 'ｰ'
            | '々'
            | 'ゝ'
            | 'ゞ'
            | 'ヽ'
            | 'ヾ'
            | '〜'
            | '～'
            | '‐'
            | '‑'
            | '–'
            | '—'
            | '―'
            | 'ぁ'
            | 'ぃ'
            | 'ぅ'
            | 'ぇ'
            | 'ぉ'
            | 'っ'
            | 'ゃ'
            | 'ゅ'
            | 'ょ'
            | 'ゎ'
            | 'ゕ'
            | 'ゖ'
            | 'ァ'
            | 'ィ'
            | 'ゥ'
            | 'ェ'
            | 'ォ'
            | 'ッ'
            | 'ャ'
            | 'ュ'
            | 'ョ'
            | 'ヮ'
            | 'ヵ'
            | 'ヶ'
            | 'ｧ'
            | 'ｨ'
            | 'ｩ'
            | 'ｪ'
            | 'ｫ'
            | 'ｯ'
            | 'ｬ'
            | 'ｭ'
            | 'ｮ'
            | 'ㇰ'..='ㇿ' | '°' | '′' | '″' | '℃' | '%' | '％' | '‰'
    )
}

pub(crate) fn is_line_end_prohibited(egc: &str) -> bool {
    let Some(ch) = single_char(egc) else {
        return false;
    };
    matches!(
        ch,
        '（' | '［'
            | '｛'
            | '「'
            | '『'
            | '【'
            | '〔'
            | '〈'
            | '《'
            | '“'
            | '‘'
            | '('
            | '['
            | '{'
            | '«'
            | '‹'
            | '￥'
            | '＄'
            | '¥'
            | '$'
    )
}

pub(crate) fn is_inseparable_pair(prev: &str, next: &str) -> bool {
    matches!(
        (single_char(prev), single_char(next)),
        (Some('\u{2026}'), Some('\u{2026}'))
            | (Some('\u{2025}'), Some('\u{2025}'))
            | (Some('\u{2014}'), Some('\u{2014}'))
            | (Some('\u{2015}'), Some('\u{2015}'))
    )
}

pub(crate) fn is_hanging_punctuation(egc: &str) -> bool {
    matches!(single_char(egc), Some('、' | '。' | '，' | '．'))
}

pub(crate) fn resolve_kinsoku_break_count(
    para_text: &str,
    units: &[Range<usize>],
    chosen_count: usize,
    hanging: bool,
) -> usize {
    if chosen_count == 0 || chosen_count >= units.len() {
        return chosen_count;
    }

    if hanging {
        let hang_unit = &para_text[units[chosen_count].clone()];
        let prev_unit = &para_text[units[chosen_count - 1].clone()];
        if is_hanging_punctuation(hang_unit) && !is_line_end_prohibited(prev_unit) {
            if chosen_count + 1 == units.len() {
                return chosen_count + 1;
            }
            let after_hang = &para_text[units[chosen_count + 1].clone()];
            if !is_line_start_prohibited(after_hang) && !is_inseparable_pair(hang_unit, after_hang)
            {
                return chosen_count + 1;
            }
        }
    }

    let mut count = chosen_count;
    while count > 0 {
        let last = &para_text[units[count - 1].clone()];
        let next = &para_text[units[count].clone()];
        if is_line_end_prohibited(last)
            || is_line_start_prohibited(next)
            || is_inseparable_pair(last, next)
        {
            count -= 1;
        } else {
            return count;
        }
    }

    chosen_count
}
