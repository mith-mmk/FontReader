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
        Ok(Self { ranges, faces })
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
/// Adjacent clusters choosing the same borrowed face share GSUB/GPOS even
/// across rule boundaries. Face changes end shaping context; cloned faces are
/// distinct selections. All faces use the same requested size and existing
/// face-specific line-metric scaling, not a shared units-per-em denominator.
/// Coordinates and advances use the engine's output units and y-down axes.
/// Per-rule scale/baseline adjustments, RTL/BiDi, vertical and multiline layout
/// are not provided. Existing family and engine behavior is unchanged. As in
/// the face engine, extremely large finite sizes/stretch or accumulated advances
/// can overflow f32 output coordinates; finite inputs do not guarantee finite output.
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
        let mut previous: Option<&FontFace> = None;
        for unit in Font::parse_text_units_for_fallback(text) {
            let ParsedTextUnit::Glyph { ch, ref text, .. } = unit else {
                unreachable!("line controls were rejected above");
            };
            let candidates = self
                .rules
                .iter()
                .find(|rule| rule.ranges.iter().any(|range| range.contains(ch)))
                .map_or(self.default_faces.as_slice(), |rule| rule.faces.as_slice());
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
            if let Some(old) = previous {
                if !std::ptr::eq(old, face) {
                    self.append_span(&mut output, &mut cursor, old, &pending)?;
                    pending.clear();
                }
            }
            previous = Some(face);
            pending.push_str(text);
        }
        if let Some(face) = previous {
            self.append_span(&mut output, &mut cursor, face, &pending)?;
        }
        Ok(output)
    }

    fn append_span(
        &self,
        output: &mut GlyphRun,
        cursor: &mut (f32, f32),
        face: &FontFace,
        text: &str,
    ) -> Result<(), Error> {
        let mut run = face
            .engine()
            .with_options(self.options.clone())
            .shape(text)?;
        let mut delta = (0.0, 0.0);
        for glyph in &mut run.glyphs {
            glyph.x += cursor.0;
            glyph.y += cursor.1;
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
