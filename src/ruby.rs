//! Japanese ruby (furigana) and emphasis-mark (bouten) layout APIs.
//!
//! Annotations address original UTF-8 byte ranges on top of an already-wrapped
//! [`WrappedLineLayout`] or [`VerticalWrappedLayout`].
use crate::{
    CaretAffinity, FontEngine, GlyphRun, TextDirection, TextPosition, VerticalWrappedLayout,
    WrappedLineLayout,
};
use std::io::{Error, ErrorKind};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// A ruby (furigana) annotation attached to a base-text UTF-8 byte range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RubyAnnotation {
    pub base_range: Range<usize>,
    pub reading: String,
}

/// Alignment policy when ruby text is shorter than the base span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RubyAlignment {
    #[default]
    Center,
    Start,
    SpaceAround,
}

/// An emphasis mark (bouten / kenten) attached to each EGC in `range`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmphasisMark {
    pub range: Range<usize>,
    pub mark: char,
}

/// Layout options for ruby and emphasis-mark placement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RubyLayoutOptions {
    pub ruby_scale: f32,
    pub ruby_gap: f32,
    pub alignment: RubyAlignment,
    pub emphasis_scale: f32,
    pub emphasis_gap: f32,
}

impl Default for RubyLayoutOptions {
    fn default() -> Self {
        Self {
            ruby_scale: 0.5,
            ruby_gap: 0.0,
            alignment: RubyAlignment::Center,
            emphasis_scale: 0.5,
            emphasis_gap: 0.0,
        }
    }
}

/// One positioned ruby run ready to draw at `(origin_x(), origin_y())`.
#[derive(Debug, Clone)]
pub struct PositionedRuby {
    base_range: Range<usize>,
    reading: String,
    run: GlyphRun,
    origin_x: f32,
    origin_y: f32,
    line_index: usize,
}

impl PositionedRuby {
    pub fn base_range(&self) -> Range<usize> {
        self.base_range.clone()
    }

    pub fn reading(&self) -> &str {
        &self.reading
    }

    pub fn glyph_run(&self) -> &GlyphRun {
        &self.run
    }

    pub fn origin_x(&self) -> f32 {
        self.origin_x
    }

    pub fn origin_y(&self) -> f32 {
        self.origin_y
    }

    pub fn line_index(&self) -> usize {
        self.line_index
    }
}

/// One positioned emphasis mark for a single EGC in an [`EmphasisMark`] span.
#[derive(Debug, Clone)]
pub struct EmphasisGlyphPlacement {
    cluster_range: Range<usize>,
    run: GlyphRun,
    origin_x: f32,
    origin_y: f32,
}

impl EmphasisGlyphPlacement {
    pub fn cluster_range(&self) -> Range<usize> {
        self.cluster_range.clone()
    }

    pub fn glyph_run(&self) -> &GlyphRun {
        &self.run
    }

    pub fn origin_x(&self) -> f32 {
        self.origin_x
    }

    pub fn origin_y(&self) -> f32 {
        self.origin_y
    }
}

/// Positioned emphasis marks grouped by visual line.
#[derive(Debug, Clone)]
pub struct PositionedEmphasis {
    range: Range<usize>,
    mark: char,
    placements: Vec<EmphasisGlyphPlacement>,
    line_index: usize,
}

impl PositionedEmphasis {
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    pub fn mark(&self) -> char {
        self.mark
    }

    pub fn placements(&self) -> &[EmphasisGlyphPlacement] {
        &self.placements
    }

    pub fn line_index(&self) -> usize {
        self.line_index
    }
}

/// Positioned ruby and emphasis-mark annotations derived from a wrapped layout.
#[derive(Debug, Clone)]
pub struct AnnotatedLineLayout {
    rubies: Vec<PositionedRuby>,
    emphases: Vec<PositionedEmphasis>,
}

impl AnnotatedLineLayout {
    pub fn rubies(&self) -> &[PositionedRuby] {
        &self.rubies
    }

    pub fn emphases(&self) -> &[PositionedEmphasis] {
        &self.emphases
    }

    /// Computes positioned ruby and emphasis marks for a [`WrappedLineLayout`].
    ///
    /// Every [`RubyAnnotation::base_range`] must lie on EGC boundaries within a
    /// single visual line; crossing a line break returns [`ErrorKind::InvalidInput`].
    pub fn layout(
        wrapped: &WrappedLineLayout,
        ruby_engine: &FontEngine<'_>,
        rubies: &[RubyAnnotation],
        emphases: &[EmphasisMark],
        options: RubyLayoutOptions,
    ) -> Result<Self, Error> {
        let line_views: Vec<LineView<'_>> = wrapped
            .lines()
            .iter()
            .map(|line| LineView {
                content: line.content_range(),
                origin_x: line.origin_x(),
                origin_y: line.origin_y(),
                glyph_run: line.shaped().glyph_run(),
                sources: line.shaped().glyph_sources(),
            })
            .collect();
        let direction = wrapped.direction();
        let line_height = wrapped.line_height();
        Self::layout_with_lines(
            wrapped.text(),
            direction,
            line_height,
            &line_views,
            &|pos| {
                wrapped
                    .caret_positions(pos)
                    .map(|v| v.into_iter().map(|c| (c.x(), c.y())).collect())
            },
            ruby_engine,
            rubies,
            emphases,
            options,
        )
    }

    /// Computes positioned ruby and emphasis marks for a [`VerticalWrappedLayout`].
    pub fn layout_vertical_wrapped(
        wrapped: &VerticalWrappedLayout,
        ruby_engine: &FontEngine<'_>,
        rubies: &[RubyAnnotation],
        emphases: &[EmphasisMark],
        options: RubyLayoutOptions,
    ) -> Result<Self, Error> {
        let line_views: Vec<LineView<'_>> = wrapped
            .lines()
            .iter()
            .map(|line| LineView {
                content: line.content_range(),
                origin_x: line.origin_x(),
                origin_y: line.origin_y(),
                glyph_run: line.layout().glyph_run(),
                sources: line.layout().glyph_sources(),
            })
            .collect();
        let line_height = wrapped.line_height();
        Self::layout_with_lines(
            wrapped.text(),
            TextDirection::TopToBottom,
            line_height,
            &line_views,
            &|pos| {
                wrapped
                    .caret_positions(pos)
                    .map(|v| v.into_iter().map(|c| (c.x(), c.y())).collect())
            },
            ruby_engine,
            rubies,
            emphases,
            options,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn layout_with_lines<F>(
        text: &str,
        direction: TextDirection,
        line_height: f32,
        lines: &[LineView<'_>],
        caret_xy: &F,
        ruby_engine: &FontEngine<'_>,
        rubies: &[RubyAnnotation],
        emphases: &[EmphasisMark],
        options: RubyLayoutOptions,
    ) -> Result<Self, Error>
    where
        F: Fn(TextPosition) -> Result<Vec<(f32, f32)>, Error>,
    {
        validate_options(&options)?;
        let egcs: Vec<Range<usize>> = text
            .grapheme_indices(true)
            .map(|(i, g)| i..i + g.len())
            .collect();
        let sorted_rubies = validate_rubies(text, &egcs, rubies)?;
        let sorted_emphases = validate_emphases(text, &egcs, emphases, &sorted_rubies)?;

        let base_font_size = ruby_engine.options().font_size;
        let ruby_size = base_font_size * options.ruby_scale;
        let emphasis_size = base_font_size * options.emphasis_scale;
        if !ruby_size.is_finite()
            || ruby_size <= 0.0
            || !emphasis_size.is_finite()
            || emphasis_size <= 0.0
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "scaled annotation font size must be positive and finite",
            ));
        }

        let ruby_shaped_engine = match direction {
            TextDirection::TopToBottom => ruby_engine
                .clone()
                .with_font_size(ruby_size)
                .with_vertical_flow(),
            TextDirection::RightToLeft => ruby_engine
                .clone()
                .with_font_size(ruby_size)
                .with_right_to_left(),
            TextDirection::LeftToRight => ruby_engine
                .clone()
                .with_font_size(ruby_size)
                .with_left_to_right(),
        };

        let mut positioned_rubies = Vec::with_capacity(sorted_rubies.len());
        for ruby in &sorted_rubies {
            let line_index = find_containing_line(lines, &ruby.base_range)?;
            let line = &lines[line_index];
            let (start_x, start_y) = first_caret_xy(
                caret_xy,
                TextPosition::new(ruby.base_range.start, CaretAffinity::Downstream),
            )?;
            let (end_x, end_y) = first_caret_xy(
                caret_xy,
                TextPosition::new(ruby.base_range.end, CaretAffinity::Upstream),
            )?;

            let mut ruby_run = ruby_shaped_engine.shape(&ruby.reading)?;
            let (origin_x, origin_y) = match direction {
                TextDirection::TopToBottom => {
                    let base_min_y = start_y.min(end_y);
                    let base_max_y = start_y.max(end_y);
                    let base_height = base_max_y - base_min_y;
                    let ruby_height: f32 = ruby_run
                        .glyphs
                        .iter()
                        .map(|g| g.glyph.metrics.advance_y.abs())
                        .sum();
                    let origin_y = match options.alignment {
                        RubyAlignment::Center => base_min_y + (base_height - ruby_height) / 2.0,
                        RubyAlignment::Start => base_min_y,
                        RubyAlignment::SpaceAround => {
                            if base_height > ruby_height && ruby_run.glyphs.len() > 1 {
                                let step =
                                    (base_height - ruby_height) / ruby_run.glyphs.len() as f32;
                                for (idx, glyph) in ruby_run.glyphs.iter_mut().enumerate() {
                                    glyph.y += step * (idx as f32 + 0.5);
                                }
                                base_min_y
                            } else {
                                base_min_y + (base_height - ruby_height) / 2.0
                            }
                        }
                    };
                    let half_em = span_half_em(line, &ruby.base_range, line_height);
                    (line.origin_x + half_em + options.ruby_gap, origin_y)
                }
                TextDirection::LeftToRight | TextDirection::RightToLeft => {
                    let base_min_x = start_x.min(end_x);
                    let base_max_x = start_x.max(end_x);
                    let base_width = base_max_x - base_min_x;
                    let ruby_width: f32 = ruby_run
                        .glyphs
                        .iter()
                        .map(|g| g.glyph.metrics.advance_x.abs())
                        .sum();
                    let origin_x = match options.alignment {
                        RubyAlignment::Center => {
                            if direction == TextDirection::RightToLeft {
                                base_max_x - (base_width - ruby_width) / 2.0
                            } else {
                                base_min_x + (base_width - ruby_width) / 2.0
                            }
                        }
                        RubyAlignment::Start => {
                            if direction == TextDirection::RightToLeft {
                                base_max_x
                            } else {
                                base_min_x
                            }
                        }
                        RubyAlignment::SpaceAround => {
                            if base_width > ruby_width && ruby_run.glyphs.len() > 1 {
                                let step = (base_width - ruby_width) / ruby_run.glyphs.len() as f32;
                                if direction == TextDirection::RightToLeft {
                                    for (idx, glyph) in ruby_run.glyphs.iter_mut().enumerate() {
                                        glyph.x -= step * (idx as f32 + 0.5);
                                    }
                                    base_max_x
                                } else {
                                    for (idx, glyph) in ruby_run.glyphs.iter_mut().enumerate() {
                                        glyph.x += step * (idx as f32 + 0.5);
                                    }
                                    base_min_x
                                }
                            } else if direction == TextDirection::RightToLeft {
                                base_max_x - (base_width - ruby_width) / 2.0
                            } else {
                                base_min_x + (base_width - ruby_width) / 2.0
                            }
                        }
                    };
                    let base_ascent = span_ascent(line, &ruby.base_range, line_height);
                    (origin_x, line.origin_y - base_ascent - options.ruby_gap)
                }
            };

            positioned_rubies.push(PositionedRuby {
                base_range: ruby.base_range.clone(),
                reading: ruby.reading.clone(),
                run: ruby_run,
                origin_x,
                origin_y,
                line_index,
            });
        }

        let emphasis_engine = match direction {
            TextDirection::TopToBottom => ruby_engine
                .clone()
                .with_font_size(emphasis_size)
                .with_vertical_flow(),
            TextDirection::RightToLeft => ruby_engine
                .clone()
                .with_font_size(emphasis_size)
                .with_right_to_left(),
            TextDirection::LeftToRight => ruby_engine
                .clone()
                .with_font_size(emphasis_size)
                .with_left_to_right(),
        };

        let mut positioned_emphases = Vec::new();
        for mark in &sorted_emphases {
            let mut mark_buf = [0u8; 4];
            let mark_str = mark.mark.encode_utf8(&mut mark_buf);
            let mark_run = emphasis_engine.shape(mark_str)?;
            let mark_width: f32 = mark_run
                .glyphs
                .iter()
                .map(|g| g.glyph.metrics.advance_x.abs())
                .sum();
            let mark_height: f32 = mark_run
                .glyphs
                .iter()
                .map(|g| g.glyph.metrics.advance_y.abs())
                .sum();

            let mut current_line: Option<usize> = None;
            let mut current_placements: Vec<EmphasisGlyphPlacement> = Vec::new();
            let mut segment_start = mark.range.start;

            for (offset, g) in text[mark.range.clone()].grapheme_indices(true) {
                let cluster_range =
                    (mark.range.start + offset)..(mark.range.start + offset + g.len());
                let line_index = find_containing_line(lines, &cluster_range)?;
                let line = &lines[line_index];
                let (start_x, start_y) = first_caret_xy(
                    caret_xy,
                    TextPosition::new(cluster_range.start, CaretAffinity::Downstream),
                )?;
                let (end_x, end_y) = first_caret_xy(
                    caret_xy,
                    TextPosition::new(cluster_range.end, CaretAffinity::Upstream),
                )?;

                let (origin_x, origin_y) = match direction {
                    TextDirection::TopToBottom => {
                        let mid_y = (start_y + end_y) / 2.0;
                        let half_em = span_half_em(line, &cluster_range, line_height);
                        (
                            line.origin_x + half_em + options.emphasis_gap,
                            mid_y - mark_height / 2.0,
                        )
                    }
                    TextDirection::LeftToRight | TextDirection::RightToLeft => {
                        let mid_x = (start_x + end_x) / 2.0;
                        let base_ascent = span_ascent(line, &cluster_range, line_height);
                        let ox = if direction == TextDirection::RightToLeft {
                            mid_x + mark_width / 2.0
                        } else {
                            mid_x - mark_width / 2.0
                        };
                        (ox, line.origin_y - base_ascent - options.emphasis_gap)
                    }
                };

                if let Some(prev_line) = current_line {
                    if prev_line != line_index {
                        let prev_end = current_placements
                            .last()
                            .map(|p| p.cluster_range.end)
                            .unwrap_or(cluster_range.start);
                        positioned_emphases.push(PositionedEmphasis {
                            range: segment_start..prev_end,
                            mark: mark.mark,
                            placements: std::mem::take(&mut current_placements),
                            line_index: prev_line,
                        });
                        segment_start = cluster_range.start;
                    }
                }
                current_line = Some(line_index);
                current_placements.push(EmphasisGlyphPlacement {
                    cluster_range,
                    run: mark_run.clone(),
                    origin_x,
                    origin_y,
                });
            }

            if let Some(line_index) = current_line {
                positioned_emphases.push(PositionedEmphasis {
                    range: segment_start..mark.range.end,
                    mark: mark.mark,
                    placements: current_placements,
                    line_index,
                });
            }
        }

        Ok(Self {
            rubies: positioned_rubies,
            emphases: positioned_emphases,
        })
    }
}

struct LineView<'a> {
    content: Range<usize>,
    origin_x: f32,
    origin_y: f32,
    glyph_run: &'a GlyphRun,
    sources: &'a [crate::GlyphSource],
}

fn validate_options(options: &RubyLayoutOptions) -> Result<(), Error> {
    if !options.ruby_scale.is_finite()
        || options.ruby_scale <= 0.0
        || !options.emphasis_scale.is_finite()
        || options.emphasis_scale <= 0.0
        || !options.ruby_gap.is_finite()
        || options.ruby_gap < 0.0
        || !options.emphasis_gap.is_finite()
        || options.emphasis_gap < 0.0
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "annotation scales must be positive/finite and gaps must be non-negative/finite",
        ));
    }
    Ok(())
}

fn check_egc_range(text: &str, egcs: &[Range<usize>], range: &Range<usize>) -> Result<(), Error> {
    if range.start >= range.end
        || range.end > text.len()
        || !text.is_char_boundary(range.start)
        || !text.is_char_boundary(range.end)
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "annotation range must be non-empty and align to UTF-8 boundaries",
        ));
    }
    if !egcs.iter().any(|e| e.start == range.start) || !egcs.iter().any(|e| e.end == range.end) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "annotation range must align to extended grapheme cluster boundaries",
        ));
    }
    Ok(())
}

fn validate_rubies(
    text: &str,
    egcs: &[Range<usize>],
    rubies: &[RubyAnnotation],
) -> Result<Vec<RubyAnnotation>, Error> {
    let mut sorted = rubies.to_vec();
    sorted.sort_by_key(|r| (r.base_range.start, r.base_range.end));
    let mut prev_end = 0usize;
    for ruby in &sorted {
        check_egc_range(text, egcs, &ruby.base_range)?;
        if ruby.base_range.start < prev_end {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "ruby annotations must not overlap",
            ));
        }
        if ruby.reading.is_empty() || ruby.reading.chars().any(|c| c.is_control()) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "ruby reading must be non-empty without control characters",
            ));
        }
        prev_end = ruby.base_range.end;
    }
    Ok(sorted)
}

fn validate_emphases(
    text: &str,
    egcs: &[Range<usize>],
    emphases: &[EmphasisMark],
    sorted_rubies: &[RubyAnnotation],
) -> Result<Vec<EmphasisMark>, Error> {
    let mut sorted = emphases.to_vec();
    sorted.sort_by_key(|e| (e.range.start, e.range.end));
    let mut prev_end = 0usize;
    for mark in &sorted {
        check_egc_range(text, egcs, &mark.range)?;
        if mark.range.start < prev_end {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "emphasis marks must not overlap",
            ));
        }
        if mark.mark.is_control() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "emphasis mark cannot be a control character",
            ));
        }
        if sorted_rubies
            .iter()
            .any(|r| r.base_range.start < mark.range.end && mark.range.start < r.base_range.end)
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "emphasis marks must not overlap ruby annotations",
            ));
        }
        prev_end = mark.range.end;
    }
    Ok(sorted)
}

fn find_containing_line(lines: &[LineView<'_>], range: &Range<usize>) -> Result<usize, Error> {
    lines
        .iter()
        .position(|line| range.start >= line.content.start && range.end <= line.content.end)
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "annotation range crosses line break or lies outside line content",
            )
        })
}

fn first_caret_xy<F>(caret_xy: &F, pos: TextPosition) -> Result<(f32, f32), Error>
where
    F: Fn(TextPosition) -> Result<Vec<(f32, f32)>, Error>,
{
    caret_xy(pos)?
        .into_iter()
        .next()
        .ok_or_else(|| Error::new(ErrorKind::Unsupported, "missing caret position"))
}

fn span_ascent(line: &LineView<'_>, global_range: &Range<usize>, line_height: f32) -> f32 {
    let local_start = global_range.start - line.content.start;
    let local_end = global_range.end - line.content.start;
    let mut max_ascent: Option<f32> = None;
    for (glyph, source) in line.glyph_run.glyphs.iter().zip(line.sources.iter()) {
        let r = source.byte_range();
        if r.start < local_end && local_start < r.end {
            if let Some(font) = glyph.glyph.font {
                max_ascent = Some(max_ascent.map_or(font.ascent, |v| v.max(font.ascent)));
            }
        }
    }
    max_ascent
        .or_else(|| {
            line.glyph_run
                .glyphs
                .first()
                .and_then(|g| g.glyph.font)
                .map(|f| f.ascent)
        })
        .unwrap_or(line_height * 0.8)
}

fn span_half_em(line: &LineView<'_>, global_range: &Range<usize>, line_height: f32) -> f32 {
    let local_start = global_range.start - line.content.start;
    let local_end = global_range.end - line.content.start;
    let mut max_half: Option<f32> = None;
    for (glyph, source) in line.glyph_run.glyphs.iter().zip(line.sources.iter()) {
        let r = source.byte_range();
        if r.start < local_end && local_start < r.end {
            if let Some(font) = glyph.glyph.font {
                let half = (font.ascent + font.descent) / 2.0;
                max_half = Some(max_half.map_or(half, |v| v.max(half)));
            }
        }
    }
    max_half
        .or_else(|| {
            line.glyph_run
                .glyphs
                .first()
                .and_then(|g| g.glyph.font)
                .map(|f| (f.ascent + f.descent) / 2.0)
        })
        .unwrap_or(line_height * 0.5)
}
