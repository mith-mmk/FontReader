//! Source-preserving mixed vertical geometry. The existing explicit LTR policy
//! is retained; this is not paragraph BiDi or mixed-direction sideways shaping.
use crate::lines::{select_nearest_line_index, split_hard_lines};
use crate::wrapped::resolve_kinsoku_break_count;
use crate::{
    CaretAffinity, CaretLayout, CaretPosition, CaretQuality, FontEngine, FontFamily, FontOptions,
    GlyphRun, GlyphSource, LineBreakKind, ShapedText, TextDirection, TextPosition, WrapMode,
};
use std::io::{Error, ErrorKind};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

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

/// One column in a [`VerticalWrappedLayout`].
#[derive(Debug, Clone)]
pub struct VerticalWrappedLine {
    content: Range<usize>,
    line_break: Range<usize>,
    break_kind: LineBreakKind,
    layout: VerticalMappedLayout,
    origin_x: f32,
    origin_y: f32,
    inline_advance: f32,
}

impl VerticalWrappedLine {
    /// Original UTF-8 byte range of this column, excluding hard-break bytes.
    pub fn content_range(&self) -> Range<usize> {
        self.content.clone()
    }

    /// Original LF or CRLF byte range when `break_kind() == LineBreakKind::Hard`.
    pub fn break_range(&self) -> Range<usize> {
        self.line_break.clone()
    }

    pub fn break_kind(&self) -> LineBreakKind {
        self.break_kind
    }

    pub fn layout(&self) -> &VerticalMappedLayout {
        &self.layout
    }

    pub fn origin_x(&self) -> f32 {
        self.origin_x
    }

    pub fn origin_y(&self) -> f32 {
        self.origin_y
    }

    pub fn inline_advance(&self) -> f32 {
        self.inline_advance
    }
}

/// Caret position in a [`VerticalWrappedLayout`], including the column index.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VerticalColumnCaretPosition {
    position: TextPosition,
    x: f32,
    y: f32,
    quality: CaretQuality,
    source: crate::CaretSource,
    line_index: usize,
    group_index: usize,
}

impl VerticalColumnCaretPosition {
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

    pub fn source(self) -> crate::CaretSource {
        self.source
    }

    pub fn line_index(self) -> usize {
        self.line_index
    }

    pub fn group_index(self) -> usize {
        self.group_index
    }
}

/// Multi-column vertical mixed layout with wrapping, Kinsoku, fallback, and
/// inline tate-chu-yoko spans.
///
/// Columns advance right-to-left (`origin_x = -col_index * line_height`).
#[derive(Debug, Clone)]
pub struct VerticalWrappedLayout {
    text: String,
    lines: Vec<VerticalWrappedLine>,
    line_height: f32,
    max_inline_extent: f32,
    wrap_mode: WrapMode,
}

struct PendingVerticalLine {
    content: Range<usize>,
    line_break: Range<usize>,
    break_kind: LineBreakKind,
    layout: VerticalMappedLayout,
    inline_advance: f32,
}

impl VerticalWrappedLayout {
    /// Shapes multi-column vertical mixed text using a single [`FontEngine`].
    pub fn shape_vertical_mixed_ltr(
        engine: &FontEngine<'_>,
        text: &str,
        max_inline_extent: f32,
        wrap_mode: WrapMode,
        tate_chu_yoko_ranges: &[Range<usize>],
    ) -> Result<Self, Error> {
        let options = engine.options();
        let line_height = options.line_height.unwrap_or(options.font_size);
        Self::shape_with(
            text,
            line_height,
            max_inline_extent,
            wrap_mode,
            tate_chu_yoko_ranges,
            &mut |slice| engine.shape_vertical_mixed_ltr_mapped(slice),
            &mut |slice| Ok((engine.shape_tate_chu_yoko_ltr(slice)?, 0)),
        )
    }

    /// Shapes multi-column vertical mixed text using [`FontFamily`] fallback.
    pub fn shape_vertical_mixed_family<'a>(
        family: &'a FontFamily,
        text: &str,
        options: FontOptions<'a>,
        max_inline_extent: f32,
        wrap_mode: WrapMode,
        tate_chu_yoko_ranges: &[Range<usize>],
    ) -> Result<Self, Error> {
        let line_height = options.line_height.unwrap_or(options.font_size);
        Self::shape_with(
            text,
            line_height,
            max_inline_extent,
            wrap_mode,
            tate_chu_yoko_ranges,
            &mut |slice| family.shape_vertical_mixed_ltr_mapped(slice, options.clone()),
            &mut |slice| family.shape_tate_chu_yoko_ltr_with_face(slice, options.clone()),
        )
    }

    fn shape_with<FM, FT>(
        text: &str,
        line_height: f32,
        max_inline_extent: f32,
        wrap_mode: WrapMode,
        tate_chu_yoko_ranges: &[Range<usize>],
        shape_mixed: &mut FM,
        shape_tcy: &mut FT,
    ) -> Result<Self, Error>
    where
        FM: FnMut(&str) -> Result<VerticalMappedLayout, Error>,
        FT: FnMut(&str) -> Result<(GlyphRun, usize), Error>,
    {
        if !max_inline_extent.is_finite() || max_inline_extent <= 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "max inline extent must be positive and finite",
            ));
        }
        shape_mixed("")?;
        if !line_height.is_finite() || line_height <= 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "line height must be positive and finite",
            ));
        }

        let validated_tcy = validate_tate_chu_yoko_ranges(text, tate_chu_yoko_ranges)?;
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
            let para_tcy: Vec<Range<usize>> = validated_tcy
                .iter()
                .filter(|r| r.start >= para_content.start && r.end <= para_content.end)
                .map(|r| (r.start - para_content.start)..(r.end - para_content.start))
                .collect();

            if validated_tcy
                .iter()
                .any(|r| r.start < para_content.end && r.end > para_content.end)
            {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "tate-chu-yoko range cannot cross a line break",
                ));
            }

            let (para_layout, para_advance) =
                shape_vertical_slice(para_text, &para_tcy, shape_mixed, shape_tcy)?;

            if para_text.is_empty() || para_advance <= max_inline_extent {
                pending.push(PendingVerticalLine {
                    content: para_content,
                    line_break: para_break,
                    break_kind: terminal_kind,
                    layout: para_layout,
                    inline_advance: para_advance,
                });
                continue;
            }

            let units = build_atomic_units(para_text, &para_tcy);
            let mut unit_idx = 0usize;

            while unit_idx < units.len() {
                let start_local = units[unit_idx].start;
                let remaining_units = units.len() - unit_idx;

                if unit_idx > 0 {
                    let rem_text = &para_text[start_local..];
                    let rem_tcy = rebase_ranges(&para_tcy, start_local..para_text.len());
                    let (rem_layout, rem_advance) =
                        shape_vertical_slice(rem_text, &rem_tcy, shape_mixed, shape_tcy)?;
                    if rem_advance <= max_inline_extent {
                        pending.push(PendingVerticalLine {
                            content: (para_content.start + start_local)..para_content.end,
                            line_break: para_break.clone(),
                            break_kind: terminal_kind,
                            layout: rem_layout,
                            inline_advance: rem_advance,
                        });
                        break;
                    }
                }

                let mut best: Option<(usize, VerticalMappedLayout, f32)> = None;
                let mut lo = 1usize;
                let mut hi = remaining_units.saturating_sub(1);
                while lo <= hi {
                    let mid = lo + (hi - lo) / 2;
                    let end_local = units[unit_idx + mid - 1].end;
                    let slice = &para_text[start_local..end_local];
                    let slice_tcy = rebase_ranges(&para_tcy, start_local..end_local);
                    let (layout, advance) =
                        shape_vertical_slice(slice, &slice_tcy, shape_mixed, shape_tcy)?;
                    if advance <= max_inline_extent {
                        best = Some((mid, layout, advance));
                        lo = mid + 1;
                    } else {
                        hi = mid.saturating_sub(1);
                    }
                }

                let (mut chosen_count, mut chosen_layout, mut chosen_advance) =
                    if let Some(found) = best {
                        found
                    } else {
                        let end_local = units[unit_idx].end;
                        let slice_tcy = rebase_ranges(&para_tcy, start_local..end_local);
                        let (layout, advance) = shape_vertical_slice(
                            &para_text[start_local..end_local],
                            &slice_tcy,
                            shape_mixed,
                            shape_tcy,
                        )?;
                        (1, layout, advance)
                    };

                if chosen_count < remaining_units {
                    let adjusted_count = match wrap_mode {
                        WrapMode::WordOrGrapheme => (1..=chosen_count)
                            .rev()
                            .find(|&count| {
                                let u = &para_text[units[unit_idx + count - 1].clone()];
                                u == " " || u == "\u{3000}"
                            })
                            .unwrap_or(chosen_count),
                        WrapMode::Grapheme => chosen_count,
                        WrapMode::JapaneseKinsoku => resolve_kinsoku_break_count(
                            para_text,
                            &units[unit_idx..],
                            chosen_count,
                            false,
                        ),
                        WrapMode::JapaneseKinsokuHanging => resolve_kinsoku_break_count(
                            para_text,
                            &units[unit_idx..],
                            chosen_count,
                            true,
                        ),
                    };
                    if adjusted_count != chosen_count {
                        let end_local = units[unit_idx + adjusted_count - 1].end;
                        let slice_tcy = rebase_ranges(&para_tcy, start_local..end_local);
                        let (layout, advance) = shape_vertical_slice(
                            &para_text[start_local..end_local],
                            &slice_tcy,
                            shape_mixed,
                            shape_tcy,
                        )?;
                        let allowed = matches!(
                            wrap_mode,
                            WrapMode::JapaneseKinsoku | WrapMode::JapaneseKinsokuHanging
                        ) || advance <= max_inline_extent;
                        if allowed {
                            chosen_count = adjusted_count;
                            chosen_layout = layout;
                            chosen_advance = advance;
                        }
                    }
                }

                let end_local = units[unit_idx + chosen_count - 1].end;
                let global_start = para_content.start + start_local;
                let global_end = para_content.start + end_local;
                let is_para_end = unit_idx + chosen_count == units.len();

                pending.push(PendingVerticalLine {
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
                    layout: chosen_layout,
                    inline_advance: chosen_advance,
                });

                unit_idx += chosen_count;
            }
        }

        let mut lines = Vec::with_capacity(pending.len());
        for (index, item) in pending.into_iter().enumerate() {
            let distance = index as f32 * line_height;
            if !distance.is_finite() {
                return Err(Error::new(ErrorKind::InvalidInput, "line origin overflow"));
            }
            let origin_x = -distance;
            let origin_y = 0.0;
            if item
                .layout
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
            lines.push(VerticalWrappedLine {
                content: item.content,
                line_break: item.line_break,
                break_kind: item.break_kind,
                layout: item.layout,
                origin_x,
                origin_y,
                inline_advance: item.inline_advance,
            });
        }

        Ok(Self {
            text: text.to_owned(),
            lines,
            line_height,
            max_inline_extent,
            wrap_mode,
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn lines(&self) -> &[VerticalWrappedLine] {
        &self.lines
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
    pub fn visual_carets(
        &self,
        byte_offset: usize,
    ) -> Result<Vec<VerticalColumnCaretPosition>, Error> {
        let matching: Vec<(usize, &VerticalWrappedLine)> = self
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
            for caret in line.layout.caret_positions(local)? {
                out.push(self.global_caret(*line_index, caret, affinity)?);
            }
        }
        Ok(out)
    }

    /// Resolves a global UTF-8 EGC boundary using its [`CaretAffinity`].
    pub fn caret_positions(
        &self,
        position: TextPosition,
    ) -> Result<Vec<VerticalColumnCaretPosition>, Error> {
        let offset = position.byte_offset();
        let matching: Vec<(usize, &VerticalWrappedLine)> = self
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
        line.layout
            .caret_positions(local)?
            .into_iter()
            .map(|caret| self.global_caret(line_index, caret, position.affinity()))
            .collect()
    }

    /// Picks the nearest column baseline on X, then the nearest vertical caret on Y.
    pub fn hit_test(&self, x: f32, y: f32) -> Result<VerticalColumnCaretPosition, Error> {
        if !x.is_finite() || !y.is_finite() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "hit-test coordinates must be finite",
            ));
        }
        let origins: Vec<(f32, f32)> = self
            .lines
            .iter()
            .map(|line| (line.origin_x, line.origin_y))
            .collect();
        let best_index = select_nearest_line_index(&origins, true, x, y);
        let line = &self.lines[best_index];
        let local = line.layout.hit_test(x - line.origin_x, y - line.origin_y)?;
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
        caret: VerticalCaretPosition,
        affinity: CaretAffinity,
    ) -> Result<VerticalColumnCaretPosition, Error> {
        let line = &self.lines[index];
        let x = caret.x() + line.origin_x;
        let y = caret.y() + line.origin_y;
        if !x.is_finite() || !y.is_finite() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "translated caret overflow",
            ));
        }
        Ok(VerticalColumnCaretPosition {
            position: TextPosition::new(
                line.content.start + caret.text_position().byte_offset(),
                affinity,
            ),
            x,
            y,
            quality: caret.quality(),
            source: caret.source(),
            line_index: index,
            group_index: caret.group_index(),
        })
    }
}

fn validate_tate_chu_yoko_ranges(
    text: &str,
    ranges: &[Range<usize>],
) -> Result<Vec<Range<usize>>, Error> {
    if ranges.is_empty() {
        return Ok(Vec::new());
    }
    let egcs: Vec<Range<usize>> = text
        .grapheme_indices(true)
        .map(|(i, g)| i..i + g.len())
        .collect();
    let mut sorted: Vec<Range<usize>> = ranges.to_vec();
    sorted.sort_by_key(|r| (r.start, r.end));
    let mut prev_end = 0usize;
    for r in &sorted {
        if r.start >= r.end || r.end > text.len() || r.start < prev_end {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "tate-chu-yoko ranges must be non-empty, in bounds, and non-overlapping",
            ));
        }
        if !text.is_char_boundary(r.start) || !text.is_char_boundary(r.end) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "tate-chu-yoko range must align to UTF-8 boundaries",
            ));
        }
        let start_ok = egcs.iter().any(|e| e.start == r.start);
        let end_ok = egcs.iter().any(|e| e.end == r.end);
        if !start_ok || !end_ok {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "tate-chu-yoko range must align to extended grapheme cluster boundaries",
            ));
        }
        prev_end = r.end;
    }
    Ok(sorted)
}

fn build_atomic_units(para_text: &str, para_tcy: &[Range<usize>]) -> Vec<Range<usize>> {
    let mut units = Vec::new();
    let mut cursor = 0usize;
    for tcy in para_tcy {
        if cursor < tcy.start {
            for (i, g) in para_text[cursor..tcy.start].grapheme_indices(true) {
                let start = cursor + i;
                units.push(start..start + g.len());
            }
        }
        units.push(tcy.clone());
        cursor = tcy.end;
    }
    if cursor < para_text.len() {
        for (i, g) in para_text[cursor..].grapheme_indices(true) {
            let start = cursor + i;
            units.push(start..start + g.len());
        }
    }
    units
}

fn rebase_ranges(ranges: &[Range<usize>], window: Range<usize>) -> Vec<Range<usize>> {
    ranges
        .iter()
        .filter(|r| r.start >= window.start && r.end <= window.end)
        .map(|r| (r.start - window.start)..(r.end - window.start))
        .collect()
}

#[derive(Default)]
struct VerticalSliceAcc {
    glyphs: Vec<crate::PositionedGlyph>,
    sources: Vec<GlyphSource>,
    groups: Vec<VerticalGroup>,
    pen_x: f32,
    pen_y: f32,
}

impl VerticalSliceAcc {
    fn append_mixed<FM>(
        &mut self,
        slice: &str,
        seg_start: usize,
        seg_end: usize,
        shape_mixed: &mut FM,
    ) -> Result<(), Error>
    where
        FM: FnMut(&str) -> Result<VerticalMappedLayout, Error>,
    {
        if seg_start >= seg_end {
            return Ok(());
        }
        let sub_text = &slice[seg_start..seg_end];
        let sub_layout = shape_mixed(sub_text)?;
        let base_group = self.groups.len();
        for mut g in sub_layout.glyph_run().glyphs.iter().cloned() {
            g.x += self.pen_x;
            g.y += self.pen_y;
            self.glyphs.push(g);
        }
        for s in sub_layout.glyph_sources() {
            let r = s.byte_range();
            self.sources.push(GlyphSource::new(
                (seg_start + r.start)..(seg_start + r.end),
                s.face_index(),
                base_group + s.run_index(),
            ));
        }
        for grp in sub_layout.groups() {
            let r = grp.byte_range();
            let t = grp.translation();
            self.groups.push(VerticalGroup::new(
                (seg_start + r.start)..(seg_start + r.end),
                grp.shaped().clone(),
                grp.is_sideways(),
                (t.0 + self.pen_x, t.1 + self.pen_y),
            ));
        }
        for g in &sub_layout.glyph_run().glyphs {
            self.pen_x += g.glyph.metrics.advance_x;
            self.pen_y += g.glyph.metrics.advance_y;
        }
        Ok(())
    }
}

fn shape_vertical_slice<FM, FT>(
    slice: &str,
    slice_tcy: &[Range<usize>],
    shape_mixed: &mut FM,
    shape_tcy: &mut FT,
) -> Result<(VerticalMappedLayout, f32), Error>
where
    FM: FnMut(&str) -> Result<VerticalMappedLayout, Error>,
    FT: FnMut(&str) -> Result<(GlyphRun, usize), Error>,
{
    if slice_tcy.is_empty() {
        let layout = shape_mixed(slice)?;
        let advance = layout
            .glyph_run()
            .glyphs
            .iter()
            .map(|g| g.glyph.metrics.advance_y)
            .sum::<f32>()
            .max(0.0);
        return Ok((layout, advance));
    }

    let mut acc = VerticalSliceAcc::default();
    let mut cursor = 0usize;

    for tcy in slice_tcy {
        if cursor < tcy.start {
            acc.append_mixed(slice, cursor, tcy.start, shape_mixed)?;
        }
        let sub_text = &slice[tcy.clone()];
        let (tcy_run, face_index) = shape_tcy(sub_text)?;
        let group_index = acc.groups.len();
        let has_glyphs = !tcy_run.glyphs.is_empty();
        let local_sources = if has_glyphs {
            vec![GlyphSource::new(0..sub_text.len(), face_index, group_index)]
        } else {
            Vec::new()
        };
        let local_shaped = ShapedText::new(
            sub_text,
            tcy_run.clone(),
            local_sources,
            TextDirection::TopToBottom,
        );
        let translation = (acc.pen_x, acc.pen_y);
        for mut g in tcy_run.glyphs {
            g.x += translation.0;
            g.y += translation.1;
            acc.pen_x += g.glyph.metrics.advance_x;
            acc.pen_y += g.glyph.metrics.advance_y;
            acc.glyphs.push(g);
        }
        acc.groups.push(VerticalGroup::new(
            tcy.clone(),
            local_shaped,
            false,
            translation,
        ));
        if has_glyphs {
            acc.sources
                .push(GlyphSource::new(tcy.clone(), face_index, group_index));
        }
        cursor = tcy.end;
    }

    if cursor < slice.len() {
        acc.append_mixed(slice, cursor, slice.len(), shape_mixed)?;
    }

    let layout =
        VerticalMappedLayout::new(slice, GlyphRun::new(acc.glyphs), acc.sources, acc.groups);
    let advance = acc.pen_y.max(0.0);
    Ok((layout, advance))
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
