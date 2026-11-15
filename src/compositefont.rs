//! Explicit character-range font selection, separate from family fallback.

use crate::fontengine::{glyph_run_to_svg, validate_single_line};
use crate::fontreader::{Font, ParsedTextUnit};
use crate::{FontFace, FontOptions, FontStretch, FontVariant, FontVariationSetting, GlyphRun};
use std::io::{Error, ErrorKind};

/// An inclusive range of Unicode scalar values (not UTF-8 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnicodeRange {
    start: char,
    end: char,
}

impl UnicodeRange {
    /// Creates an inclusive range; reversed endpoints return `InvalidInput`.
    pub fn new(start: char, end: char) -> Result<Self, Error> {
        if start > end {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Unicode range is reversed",
            ));
        }
        Ok(Self { start, end })
    }

    /// Returns the inclusive lower endpoint.
    pub fn start(&self) -> char {
        self.start
    }

    /// Returns the inclusive upper endpoint.
    pub fn end(&self) -> char {
        self.end
    }

    /// Tests one Unicode scalar value.
    pub fn contains(&self, ch: char) -> bool {
        self.start <= ch && ch <= self.end
    }
}

/// Unicode scalar ranges mapped to an ordered, nonempty list of borrowed faces.
#[derive(Debug, Clone)]
pub struct CompositeFontRule<'a> {
    ranges: Vec<UnicodeRange>,
    faces: Vec<&'a FontFace>,
    size_multiplier: f32,
    baseline_shift: f32,
}

impl<'a> CompositeFontRule<'a> {
    /// Creates a rule; empty ranges or faces return `InvalidInput`.
    /// Overlapping ranges are allowed. The first matching rule owns a cluster.
    pub fn new(ranges: Vec<UnicodeRange>, faces: Vec<&'a FontFace>) -> Result<Self, Error> {
        if ranges.is_empty() || faces.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "composite rule requires ranges and faces",
            ));
        }
        Ok(Self {
            ranges,
            faces,
            size_multiplier: 1.0,
            baseline_shift: 0.0,
        })
    }

    /// Multiplies the composition's nominal size before shaping this rule's spans.
    ///
    /// Must be positive and finite; the default is `1.0`. This uses the existing
    /// face-specific line-metric scaling, not units-per-em sizing. The effective
    /// size must also remain positive and finite when a selected span is shaped.
    pub fn with_size_multiplier(mut self, multiplier: f32) -> Result<Self, Error> {
        if !multiplier.is_finite() || multiplier <= 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "composite size multiplier must be positive and finite",
            ));
        }
        self.size_multiplier = multiplier;
        Ok(self)
    }

    /// Moves this rule's glyphs in output units; positive means up in horizontal LTR.
    ///
    /// Must be finite; the default is `0.0`. This offset is not multiplied by
    /// font size or the size multiplier. It does not change advances, local
    /// glyph bounds, or the cursor used by subsequent spans.
    pub fn with_baseline_shift(mut self, shift: f32) -> Result<Self, Error> {
        if !shift.is_finite() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "composite baseline shift must be finite",
            ));
        }
        self.baseline_shift = shift;
        Ok(self)
    }

    /// Returns the relative nominal size, defaulting to `1.0`.
    pub fn size_multiplier(&self) -> f32 {
        self.size_multiplier
    }

    /// Returns the positive-up baseline shift in output units, defaulting to `0.0`.
    pub fn baseline_shift(&self) -> f32 {
        self.baseline_shift
    }

    /// Returns the inclusive ranges.
    pub fn ranges(&self) -> &[UnicodeRange] {
        &self.ranges
    }

    /// Returns faces in fallback order.
    pub fn faces(&self) -> &[&'a FontFace] {
        &self.faces
    }
}

/// Intentional character-range selection for a single horizontal LTR line.
///
/// Before shaping, the first matching rule owns each Unicode 17.0 extended
/// grapheme cluster. Its representative is the first scalar after the existing
/// parser removes leading orphan variation selectors, not necessarily a base
/// character (Prepend characters remain representatives). Other scalars need
/// not match the range. Marks, variation sequences and ZWJ sequences stay on
/// one face. Existing parser handling of layout controls is retained.
///
/// Within the owning rule, the first face passing the existing whole-cluster
/// support check wins. If none passes, its first face keeps missing glyphs;
/// selection never escapes to a later rule or the default list. Unmatched
/// clusters use the default list with the same policy. This support check is
/// not a guarantee of complete Indic or emoji shaping. A base glyph can pass
/// when a requested variation selector has no mapping; later UVS/color faces
/// do not receive preference. Unlike [`crate::FontFamily`],
/// color preference and face continuity never override the explicit order.
///
/// Adjacent clusters choosing the same borrowed face and identical size/baseline
/// adjustments share GSUB/GPOS even across rule boundaries. Changes to either
/// the face or adjustments end shaping context; cloned faces are distinct.
/// Rule size multipliers apply before shaping, using existing face-specific
/// line-metric scaling, not a shared units-per-em denominator. Baseline shifts
/// are output units, positive up; they never change advances or the next cursor.
/// Unmatched clusters use multiplier 1 and shift 0.
///
/// Coordinates and advances use the engine's output units and y-down axes.
/// RTL/BiDi, vertical and multiline layout are future work, not provided here.
/// Existing family and engine behavior is unchanged. Runs selecting nondefault
/// adjustments reject nonfinite geometry/advances with `InvalidInput`, rather
/// than returning partial output. Unadjusted runs retain the existing engine's
/// extreme-size overflow behavior.
///
/// ```no_run
/// use fontcore::{CompositeFont, CompositeFontRule, FontFile, UnicodeRange};
/// let japanese = FontFile::from_file("fonts/Japanese.ttf")?.current_face()?;
/// let latin = FontFile::from_file("fonts/Latin.ttf")?.current_face()?;
/// let mut composite = CompositeFont::new(vec![&japanese])?.with_font_size(32.0);
/// composite.add_rule(CompositeFontRule::new(
///     vec![UnicodeRange::new(' ', '~')?], vec![&latin, &japanese],
/// )?);
/// let run = composite.shape_ltr("日本語 ABC")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct CompositeFont<'a> {
    default_faces: Vec<&'a FontFace>,
    rules: Vec<CompositeFontRule<'a>>,
    options: FontOptions<'a>,
    svg_unit: String,
}

impl<'a> CompositeFont<'a> {
    /// Creates a composition; an empty default face list returns `InvalidInput`.
    pub fn new(default_faces: Vec<&'a FontFace>) -> Result<Self, Error> {
        let first = default_faces.first().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "composite font requires default faces",
            )
        })?;
        let options = FontOptions::new(first);
        Ok(Self {
            default_faces,
            rules: Vec::new(),
            options,
            svg_unit: "px".into(),
        })
    }

    /// Appends a rule. Earlier matching rules take precedence.
    pub fn add_rule(&mut self, rule: CompositeFontRule<'a>) -> &mut Self {
        self.rules.push(rule);
        self
    }

    /// Returns rules in precedence order.
    pub fn rules(&self) -> &[CompositeFontRule<'a>] {
        &self.rules
    }

    /// Returns the ordered faces used for unmatched clusters.
    pub fn default_faces(&self) -> &[&'a FontFace] {
        &self.default_faces
    }

    /// Sets the common nominal font size, with existing per-face scaling.
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.options = self.options.with_font_size(size);
        self
    }

    /// Sets the common horizontal stretch (not a per-rule scale).
    pub fn with_font_stretch(mut self, stretch: FontStretch) -> Self {
        self.options = self.options.with_font_stretch(stretch);
        self
    }

    /// Sets the locale used by support checks and shaping.
    pub fn with_locale(mut self, locale: &'a str) -> Self {
        self.options = self.options.with_locale(locale);
        self
    }

    /// Sets the common GSUB variant.
    pub fn with_font_variant(mut self, variant: FontVariant) -> Self {
        self.options = self.options.with_font_variant(variant);
        self
    }

    /// Sets one variable-font axis, applied where supported by each face.
    pub fn with_variation(mut self, tag: &str, value: f32) -> Self {
        self.options = self.options.with_variation(tag, value);
        self
    }

    /// Replaces the variable-font axis settings.
    pub fn with_variations(mut self, variations: &[FontVariationSetting]) -> Self {
        self.options = self.options.with_variations(variations);
        self
    }

    /// Sets the SVG unit; the default is `px`.
    pub fn with_svg_unit(mut self, unit: impl Into<String>) -> Self {
        self.svg_unit = unit.into();
        self
    }

    /// Shapes one horizontal LTR line using explicit range selection.
    ///
    /// TAB, CR, LF, VT, FF, NEL, U+2028 and U+2029 return `InvalidInput`, with
    /// no partial result. Normal size/stretch validation also applies to empty
    /// text. Both x and y advances carry into subsequent face spans.
    pub fn shape_ltr(&self, text: &str) -> Result<GlyphRun, Error> {
        validate_single_line(text, "composite LTR")?;
        // Reuse normal option validation, including for empty text.
        let mut output = self.default_faces[0]
            .engine()
            .with_options(self.options.clone())
            .shape("")?;
        let mut cursor = (0.0, 0.0);
        let mut pending = String::new();
        let mut previous: Option<SpanSelection<'_>> = None;
        let mut adjusted = false;
        for unit in Font::parse_text_units_for_fallback(text) {
            let ParsedTextUnit::Glyph { ch, ref text, .. } = unit else {
                unreachable!("line controls were rejected above");
            };
            let rule = self
                .rules
                .iter()
                .find(|rule| rule.ranges.iter().any(|range| range.contains(ch)));
            let candidates =
                rule.map_or(self.default_faces.as_slice(), |rule| rule.faces.as_slice());
            let face = candidates
                .iter()
                .copied()
                .find(|face| {
                    face.font().supports_text_unit(
                        &unit,
                        self.options.text_direction,
                        self.options.locale,
                        self.options.font_variant,
                    )
                })
                .unwrap_or(candidates[0]);
            let selection = SpanSelection {
                face,
                size_multiplier: rule.map_or(1.0, |rule| rule.size_multiplier),
                baseline_shift: rule.map_or(0.0, |rule| rule.baseline_shift),
            };
            adjusted |= selection.size_multiplier != 1.0 || selection.baseline_shift != 0.0;
            if let Some(old) = previous {
                if !old.same_as(selection) {
                    self.append_span(&mut output, &mut cursor, old, &pending)?;
                    pending.clear();
                }
            }
            previous = Some(selection);
            pending.push_str(text);
        }
        if let Some(face) = previous {
            self.append_span(&mut output, &mut cursor, face, &pending)?;
        }
        if adjusted {
            validate_adjusted_run(&output)?;
        }
        Ok(output)
    }

    fn append_span(
        &self,
        output: &mut GlyphRun,
        cursor: &mut (f32, f32),
        selection: SpanSelection<'_>,
        text: &str,
    ) -> Result<(), Error> {
        let size = self.options.font_size * selection.size_multiplier;
        if !size.is_finite() || size <= 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "composite effective size must be positive and finite",
            ));
        }
        let mut run = selection
            .face
            .engine()
            .with_options(self.options.clone().with_font_size(size))
            .shape(text)?;
        let mut delta = (0.0, 0.0);
        for glyph in &mut run.glyphs {
            glyph.x += cursor.0;
            glyph.y += cursor.1;
            glyph.y -= selection.baseline_shift;
            delta.0 += glyph.glyph.metrics.advance_x;
            delta.1 += glyph.glyph.metrics.advance_y;
        }
        cursor.0 += delta.0;
        cursor.1 += delta.1;
        output.glyphs.extend(run.glyphs);
        Ok(())
    }

    /// Measures the sum of horizontal advances from [`Self::shape_ltr`], not ink width.
    pub fn measure_ltr(&self, text: &str) -> Result<f64, Error> {
        Ok(self
            .shape_ltr(text)?
            .glyphs
            .iter()
            .map(|glyph| glyph.glyph.metrics.advance_x)
            .sum::<f32>() as f64)
    }

    /// Exports the final [`Self::shape_ltr`] run with normal SVG ink padding.
    pub fn render_svg_ltr(&self, text: &str) -> Result<String, Error> {
        glyph_run_to_svg(&self.shape_ltr(text)?, &self.svg_unit)
    }
}

#[derive(Clone, Copy)]
struct SpanSelection<'a> {
    face: &'a FontFace,
    size_multiplier: f32,
    baseline_shift: f32,
}

impl SpanSelection<'_> {
    fn same_as(self, other: Self) -> bool {
        std::ptr::eq(self.face, other.face)
            && self.size_multiplier == other.size_multiplier
            && self.baseline_shift == other.baseline_shift
    }
}

// Confined to compositions that actually selected adjustments: legacy face,
// family and unadjusted composite numeric behavior is intentionally unchanged.
fn validate_adjusted_run(run: &GlyphRun) -> Result<(), Error> {
    use crate::commands::{Command, GlyphLayer, GlyphPaint};
    fn finite(values: &[f32]) -> Result<(), Error> {
        if values.iter().all(|v| v.is_finite()) {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::InvalidInput,
                "composite adjustments require finite output geometry",
            ))
        }
    }
    let mut extent: Option<(f32, f32, f32, f32)> = None;
    let mut point = |x: f32, y: f32| -> Result<(), Error> {
        finite(&[x, y])?;
        extent = Some(match extent {
            Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)),
            None => (x, y, x, y),
        });
        Ok(())
    };
    let mut advance = (0.0, 0.0);
    for positioned in &run.glyphs {
        let glyph = &positioned.glyph;
        finite(&[
            positioned.x,
            positioned.y,
            glyph.metrics.advance_x,
            glyph.metrics.advance_y,
            glyph.metrics.bearing_x,
            glyph.metrics.bearing_y,
        ])?;
        advance.0 += glyph.metrics.advance_x;
        advance.1 += glyph.metrics.advance_y;
        finite(&[advance.0, advance.1])?;
        if let Some(font) = glyph.font {
            finite(&[font.ascent, font.descent, font.line_gap])?;
        }
        if let Some(bounds) = glyph.metrics.bounds {
            finite(&[bounds.min_x, bounds.min_y, bounds.max_x, bounds.max_y])?;
            point(positioned.x + bounds.min_x, positioned.y + bounds.min_y)?;
            point(positioned.x + bounds.max_x, positioned.y + bounds.max_y)?;
        }
        for layer in &glyph.layers {
            match layer {
                GlyphLayer::Path(path) => {
                    finite(&[path.offset_x, path.offset_y, path.stroke_width])?;
                    let x = positioned.x + path.offset_x;
                    let y = positioned.y + path.offset_y;
                    finite(&[x, y])?;
                    let padding =
                        if matches!(path.paint_mode, crate::commands::PathPaintMode::Stroke) {
                            path.stroke_width.abs() / 2.0
                        } else {
                            0.0
                        };
                    let mut check_point = |px: f32, py: f32| -> Result<(), Error> {
                        finite(&[px, py])?;
                        point(x + px - padding, y + py - padding)?;
                        point(x + px + padding, y + py + padding)
                    };
                    for command in path.commands.iter().chain(&path.clip_commands) {
                        match *command {
                            Command::MoveTo(x, y) | Command::Line(x, y) => check_point(x, y)?,
                            Command::Bezier(a, b) => {
                                check_point(a.0, a.1)?;
                                check_point(b.0, b.1)?;
                            }
                            Command::CubicBezier(a, b, c) => {
                                check_point(a.0, a.1)?;
                                check_point(b.0, b.1)?;
                                check_point(c.0, c.1)?;
                            }
                            Command::Close => {}
                        }
                    }
                    match &path.paint {
                        GlyphPaint::LinearGradient(g) => {
                            finite(&[g.x1, g.y1, g.x2, g.y2])?;
                            finite(&g.transform)?;
                            for stop in &g.stops {
                                finite(&[stop.offset])?;
                            }
                        }
                        GlyphPaint::RadialGradient(g) => {
                            finite(&[g.cx, g.cy, g.r, g.fx, g.fy])?;
                            finite(&g.transform)?;
                            for stop in &g.stops {
                                finite(&[stop.offset])?;
                            }
                        }
                        _ => {}
                    }
                }
                GlyphLayer::Raster(raster) => {
                    finite(&[raster.offset_x, raster.offset_y])?;
                    let x = positioned.x + raster.offset_x;
                    let y = positioned.y + raster.offset_y;
                    point(x, y)?;
                    point(
                        x + raster.width.unwrap_or(0) as f32,
                        y + raster.height.unwrap_or(0) as f32,
                    )?;
                }
                #[cfg(feature = "svg-fonts")]
                GlyphLayer::Svg(svg) => {
                    finite(&[
                        svg.view_box_min_x,
                        svg.view_box_min_y,
                        svg.view_box_width,
                        svg.view_box_height,
                        svg.width,
                        svg.height,
                        svg.offset_x,
                        svg.offset_y,
                    ])?;
                    let x = positioned.x + svg.offset_x;
                    let y = positioned.y + svg.offset_y;
                    point(x, y)?;
                    point(x + svg.width, y + svg.height)?;
                }
            }
        }
    }
    if let Some((min_x, min_y, max_x, max_y)) = extent {
        // Include the existing SVG exporter's four-unit padding on every side.
        finite(&[
            min_x - 4.0,
            min_y - 4.0,
            max_x - min_x + 8.0,
            max_y - min_y + 8.0,
        ])?;
    }
    Ok(())
}
