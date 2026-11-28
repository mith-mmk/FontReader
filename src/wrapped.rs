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
    CaretAffinity, CaretLayout, CaretPosition, CompositeFont, FontEngine, FontFamily, FontOptions,
    LineCaretPosition, ShapedText, TextDirection, TextPosition,
};
use std::io::{Error, ErrorKind};
use std::ops::Range;

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
}

impl WrappedLineLayout {
    /// Shapes and wraps `text` using a single [`FontEngine`].
    pub fn shape(
        engine: &FontEngine<'_>,
        text: &str,
        max_inline_extent: f32,
        wrap_mode: WrapMode,
    ) -> Result<Self, Error> {
        let options = engine.options();
        let line_height = options.line_height.unwrap_or(options.font_size);
        let direction = options.text_direction;
        Self::shape_with(
            text,
            direction,
            line_height,
            max_inline_extent,
            wrap_mode,
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
        let line_height = options.line_height.unwrap_or(options.font_size);
        let direction = options.text_direction;
        Self::shape_with(
            text,
            direction,
            line_height,
            max_inline_extent,
            wrap_mode,
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
        Self::shape_with(
            text,
            TextDirection::LeftToRight,
            line_height,
            max_inline_extent,
            wrap_mode,
            |slice| composite.shape_mapped_ltr(slice),
        )
    }

    fn shape_with<F>(
        text: &str,
        direction: TextDirection,
        line_height: f32,
        max_inline_extent: f32,
        wrap_mode: WrapMode,
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
                shape_and_measure(&mut shape_line, para_text, direction)?;

            if para_text.is_empty() || para_advance <= max_inline_extent {
                pending.push(PendingVisualLine {
                    content: para_content,
                    line_break: para_break,
                    break_kind: terminal_kind,
                    shaped: para_shaped,
                    carets: para_carets,
                    inline_advance: para_advance,
                });
                continue;
            }

            let egcs: Vec<Range<usize>> = para_shaped.grapheme_ranges().to_vec();
            let mut egc_idx = 0usize;

            while egc_idx < egcs.len() {
                let start_local = egcs[egc_idx].start;
                let remaining_egcs = egcs.len() - egc_idx;

                if egc_idx > 0 {
                    let rem_text = &para_text[start_local..];
                    let (rem_shaped, rem_carets, rem_advance) =
                        shape_and_measure(&mut shape_line, rem_text, direction)?;
                    if rem_advance <= max_inline_extent {
                        pending.push(PendingVisualLine {
                            content: (para_content.start + start_local)..para_content.end,
                            line_break: para_break.clone(),
                            break_kind: terminal_kind,
                            shaped: rem_shaped,
                            carets: rem_carets,
                            inline_advance: rem_advance,
                        });
                        break;
                    }
                }

                // Binary search the largest EGC count in [1, remaining_egcs - 1]
                // whose shaped inline advance fits within max_inline_extent.
                let mut best: Option<(usize, ShapedText, CaretLayout, f32)> = None;
                let mut lo = 1usize;
                let mut hi = remaining_egcs.saturating_sub(1);
                while lo <= hi {
                    let mid = lo + (hi - lo) / 2;
                    let end_local = egcs[egc_idx + mid - 1].end;
                    let slice = &para_text[start_local..end_local];
                    let (shaped, carets, advance) =
                        shape_and_measure(&mut shape_line, slice, direction)?;
                    if advance <= max_inline_extent {
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
                        // Single EGC exceeds max_inline_extent: emit 1 EGC to guarantee progress.
                        let end_local = egcs[egc_idx].end;
                        let (shaped, carets, advance) = shape_and_measure(
                            &mut shape_line,
                            &para_text[start_local..end_local],
                            direction,
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
                        )?;
                        let allowed = matches!(
                            wrap_mode,
                            WrapMode::JapaneseKinsoku | WrapMode::JapaneseKinsokuHanging
                        ) || advance <= max_inline_extent;
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
                });

                egc_idx += chosen_count;
            }
        }

        let mut lines = Vec::with_capacity(pending.len());
        for (index, item) in pending.into_iter().enumerate() {
            let distance = index as f32 * line_height;
            if !distance.is_finite() {
                return Err(Error::new(ErrorKind::InvalidInput, "line origin overflow"));
            }
            let (origin_x, origin_y) = if direction == TextDirection::TopToBottom {
                (-distance, 0.0)
            } else {
                (0.0, distance)
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
            line.carets.hit_test(0.0, y)?
        } else {
            line.carets.hit_test(x, 0.0)?
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
) -> Result<(ShapedText, CaretLayout, f32), Error>
where
    F: FnMut(&str) -> Result<ShapedText, Error>,
{
    let shaped = shape_line(slice)?;
    let carets = CaretLayout::new(&shaped)?;
    let advance = caret_inline_advance(&carets, direction);
    Ok((shaped, carets, advance))
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
