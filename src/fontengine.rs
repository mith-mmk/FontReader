//! High-level shaping and rendering engine bound to one [`crate::FontFace`].

use crate::commands::{
    Command, FillRule, FontOptions, FontVariant, FontVariationSetting, GlyphBounds,
    GlyphGradientSpread, GlyphGradientUnits, GlyphLayer, GlyphLinearGradient, GlyphPaint,
    GlyphRadialGradient, GlyphRun, PathPaintMode, PositionedGlyph, RasterGlyphLayer,
    RasterGlyphSource, TextDirection,
};
use crate::fontface::FontFace;
use crate::util;
use base64::{engine::general_purpose, Engine as _};
use std::fmt::Write as _;
use std::io::{Error, ErrorKind};

/// Public shaping direction used by [`FontEngine`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapingPolicy {
    LeftToRight,
    RightToLeft,
    TopToBottom,
}

/// Logical and ink extents produced from one shaped glyph run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutMetrics {
    pub inline_advance: f64,
    pub block_extent: f64,
    pub ink_bounds: Option<GlyphBounds>,
    pub line_bounds: Option<GlyphBounds>,
}

impl Default for ShapingPolicy {
    fn default() -> Self {
        Self::LeftToRight
    }
}

impl ShapingPolicy {
    /// Converts the policy into the lower-level text direction.
    pub fn text_direction(self) -> TextDirection {
        match self {
            ShapingPolicy::LeftToRight => TextDirection::LeftToRight,
            ShapingPolicy::RightToLeft => TextDirection::RightToLeft,
            ShapingPolicy::TopToBottom => TextDirection::TopToBottom,
        }
    }

    fn from_text_direction(text_direction: TextDirection) -> Self {
        match text_direction {
            TextDirection::LeftToRight => ShapingPolicy::LeftToRight,
            TextDirection::RightToLeft => ShapingPolicy::RightToLeft,
            TextDirection::TopToBottom => ShapingPolicy::TopToBottom,
        }
    }
}

/// High-level builder for shaping, measuring, and SVG rendering.
#[derive(Clone)]
pub struct FontEngine<'a> {
    face: &'a FontFace,
    options: FontOptions<'a>,
    shaping_policy: ShapingPolicy,
    svg_unit: String,
}

impl<'a> FontEngine<'a> {
    /// Creates a new engine bound to one face.
    pub fn new(face: &'a FontFace) -> Self {
        Self {
            face,
            options: FontOptions::new(face),
            shaping_policy: ShapingPolicy::default(),
            svg_unit: "px".to_string(),
        }
    }

    /// Replaces the underlying [`FontOptions`].
    pub fn with_options(mut self, options: FontOptions<'a>) -> Self {
        self.shaping_policy = ShapingPolicy::from_text_direction(options.text_direction);
        self.options = options.with_face(self.face);
        self
    }

    /// Sets the font size used for shaping and rendering.
    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.options = self.options.with_font_size(font_size);
        self
    }

    /// Sets the line height used for layout.
    pub fn with_line_height(mut self, line_height: f32) -> Self {
        self.options = self.options.with_line_height(line_height);
        self
    }

    /// Sets the locale used for GSUB/GPOS lookup.
    pub fn with_locale(mut self, locale: &'a str) -> Self {
        self.options = self.options.with_locale(locale);
        self
    }

    /// Sets the GSUB variant feature selection.
    pub fn with_font_variant(mut self, font_variant: FontVariant) -> Self {
        self.options = self.options.with_font_variant(font_variant);
        self
    }

    /// Sets one variable-font axis value such as `wght=700`.
    pub fn with_variation(mut self, tag: &str, value: f32) -> Self {
        self.options = self.options.with_variation(tag, value);
        self
    }

    /// Replaces the current variable-font axis settings.
    pub fn with_variations(mut self, variations: &[FontVariationSetting]) -> Self {
        self.options = self.options.with_variations(variations);
        self
    }

    /// Clears all variable-font axis settings and returns to the default instance.
    pub fn clear_variations(mut self) -> Self {
        self.options = self.options.clear_variations();
        self
    }

    /// Convenience shorthand for `jp78`.
    pub fn with_jis78(self) -> Self {
        self.with_font_variant(FontVariant::Jis78)
    }

    /// Convenience shorthand for `jp90`.
    pub fn with_jis90(self) -> Self {
        self.with_font_variant(FontVariant::Jis90)
    }

    /// Convenience shorthand for `trad`.
    pub fn with_traditional_forms(self) -> Self {
        self.with_font_variant(FontVariant::TraditionalForms)
    }

    /// Convenience shorthand for `nlck`.
    pub fn with_nlc_kanji_forms(self) -> Self {
        self.with_font_variant(FontVariant::NlcKanjiForms)
    }

    /// Sets the shaping policy used by the engine.
    pub fn with_shaping_policy(mut self, shaping_policy: ShapingPolicy) -> Self {
        self.shaping_policy = shaping_policy;
        self.options = self
            .options
            .with_text_direction(self.shaping_policy.text_direction());
        self
    }

    /// Shorthand for left-to-right shaping.
    pub fn with_left_to_right(self) -> Self {
        self.with_shaping_policy(ShapingPolicy::LeftToRight)
    }

    /// Shorthand for right-to-left shaping.
    pub fn with_right_to_left(self) -> Self {
        self.with_shaping_policy(ShapingPolicy::RightToLeft)
    }

    /// Shorthand for top-to-bottom shaping.
    pub fn with_vertical_flow(self) -> Self {
        self.with_shaping_policy(ShapingPolicy::TopToBottom)
    }

    /// Sets the SVG unit string such as `"px"` or `"pt"`.
    pub fn with_svg_unit(mut self, unit: impl Into<String>) -> Self {
        self.svg_unit = unit.into();
        self
    }

    /// Returns the currently selected shaping policy.
    pub fn shaping_policy(&self) -> ShapingPolicy {
        self.shaping_policy
    }

    /// Returns the currently selected font variant.
    pub fn font_variant(&self) -> FontVariant {
        self.options.font_variant
    }

    /// Returns the currently selected variable-font axis settings.
    pub fn variation_settings(&self) -> &[FontVariationSetting] {
        &self.options.variations
    }

    /// Returns the effective options used by this engine.
    pub fn options(&self) -> FontOptions<'a> {
        self.options
            .clone()
            .with_text_direction(self.shaping_policy.text_direction())
    }

    /// Shapes text into a [`GlyphRun`].
    pub fn shape(&self, text: &str) -> Result<GlyphRun, Error> {
        self.text2glyph_run(text)
    }

    /// Shapes one horizontal LTR line, then rotates it 90 degrees clockwise.
    ///
    /// This per-call override ignores the engine's current direction without
    /// changing its options. Horizontal metrics, kerning, ligatures, marks,
    /// size, stretch, locale, variants, and supported variations are retained.
    /// With `layout`, only `vrtr` is applied after ordinary GSUB, before metrics
    /// and GPOS. `vert` and `vrt2` are never selected by this method.
    ///
    /// In y-down coordinates the transform is `(x, y) -> (-y, x)`, about
    /// baseline `(0, 0)`, with no em-centering translation. Positions, paths,
    /// offsets, advance vectors and bounds are rotated. Font metrics keep
    /// their horizontal ascent/descent/line gap, with flow set to vertical
    /// (ascent on the right, descent on the left). The legacy LSB/positive-up
    /// ascender bearing reference becomes `(bearing_y, -bearing_x)`; use
    /// bounds for actual ink extents rather than interpreting it as ink.
    ///
    /// Only current-color, clip-free fill paths are supported. Other paints,
    /// strokes, clips, raster or raw SVG layers return `Unsupported`, with no
    /// partial run. TAB, CR, LF, VT, FF, NEL, U+2028 and U+2029 return
    /// `InvalidInput`. This is a single-face LTR operation, not BiDi or Unicode
    /// mixed-orientation layout. Existing horizontal shaping limits apply.
    pub fn shape_sideways_ltr(&self, text: &str) -> Result<GlyphRun, Error> {
        validate_single_line(text, "sideways LTR")?;
        let run = self
            .face
            .font()
            .text2glyph_run_sideways_ltr(text, &self.options())?;
        rotate_sideways_ltr(run)
    }

    /// Measures the absolute sum of the rotated run's y advances.
    pub fn measure_sideways_ltr(&self, text: &str) -> Result<f64, Error> {
        Ok(self.measure_layout_sideways_ltr(text)?.inline_advance)
    }

    /// Measures the same rotated run returned by [`Self::shape_sideways_ltr`].
    ///
    /// Ink bounds union the positioned glyph bounds. Line bounds union every
    /// positioned origin and its full advance-vector endpoint; block extent
    /// is their x extent (not a font line height). Empty runs have no bounds
    /// and zero extents; spaces retain their horizontal advance, rotated down.
    pub fn measure_layout_sideways_ltr(&self, text: &str) -> Result<LayoutMetrics, Error> {
        let run = self.shape_sideways_ltr(text)?;
        Ok(measure_vertical_run(&run))
    }

    /// Exports [`Self::shape_sideways_ltr`] with the normal SVG ink padding.
    pub fn render_svg_sideways_ltr(&self, text: &str) -> Result<String, Error> {
        glyph_run_to_svg(&self.shape_sideways_ltr(text)?, &self.svg_unit)
    }

    /// Shapes one single-face LTR line with opt-in mixed vertical orientation.
    ///
    /// Before GSUB, Unicode 17.0 extended grapheme clusters are classified by
    /// their first scalar's Unicode 18.0 `Vertical_Orientation`. Any Unicode
    /// 18.0 enclosing mark (`Me`) forces the whole cluster upright. `U`, `Tu`,
    /// and `Tr` stay upright with only `vert`; `R` uses horizontal shaping with
    /// only `vrtr`, then a clockwise whole-group rotation. Adjacent clusters of
    /// the same final orientation share GSUB/GPOS; orientation boundaries do not.
    /// `Tr` without a vertical alternate stays upright: rotated fallback synthesis
    /// is not implemented. This is an explicit UAX #50-style policy, not complete
    /// CSS character tailoring or browser-equivalent layout.
    ///
    /// Upright glyphs retain existing vertical origins. Each sideways group is
    /// centered using its already-scaled horizontal font metrics:
    /// `x_shift = -(ascent - descent) / 2`, with positive descent. This placement
    /// shift does not affect the pen. Groups join only by the sum of their final
    /// advance vectors, never GPOS placement, ink edges, or the centering shift.
    /// This synthesis does not implement OpenType BASE or CSS baseline rules.
    ///
    /// Like [`Self::shape_sideways_ltr`], this overrides the engine's direction
    /// for this call only, accepts only current-color, clip-free fill paths,
    /// rejects tabs/line-break controls, and fails without a partial run.
    /// Existing LTR shaping and variation support limits apply. No BiDi, family
    /// fallback, multiline layout, or full OpenType shaping is provided.
    pub fn shape_vertical_mixed_ltr(&self, text: &str) -> Result<GlyphRun, Error> {
        validate_single_line(text, "mixed vertical LTR")?;
        let options = self.options();
        let font = self.face.font();
        let mut glyphs = Vec::new();
        let (mut pen_x, mut pen_y) = (0.0, 0.0);
        let groups = mixed_orientation_groups(text);
        if groups.is_empty() {
            // Preserve the underlying option validation even for empty text.
            return font.text2glyph_run_mixed_upright(text, &options);
        }
        for (group, sideways) in groups {
            let (mut run, shift_x) = if sideways {
                let horizontal = font.text2glyph_run_sideways_ltr(group, &options)?;
                // Take metrics from this shaped instance before rotation. No
                // second stretch/size scaling, line gap, or ink bounds enter it.
                let shift = horizontal
                    .glyphs
                    .first()
                    .and_then(|glyph| glyph.glyph.font)
                    .map(|metrics| -(metrics.ascent - metrics.descent) / 2.0)
                    .unwrap_or(0.0);
                (rotate_sideways_ltr(horizontal)?, shift)
            } else {
                (font.text2glyph_run_mixed_upright(group, &options)?, 0.0)
            };
            validate_outline_run(&run, "mixed vertical LTR")?;
            let (mut advance_x, mut advance_y) = (0.0, 0.0);
            for glyph in &mut run.glyphs {
                glyph.x += pen_x + shift_x;
                glyph.y += pen_y;
                advance_x += glyph.glyph.metrics.advance_x;
                advance_y += glyph.glyph.metrics.advance_y;
            }
            pen_x += advance_x;
            pen_y += advance_y;
            glyphs.extend(run.glyphs);
        }
        Ok(GlyphRun::new(glyphs))
    }

    /// Measures the absolute sum of the mixed run's final y advances.
    pub fn measure_vertical_mixed_ltr(&self, text: &str) -> Result<f64, Error> {
        Ok(self.measure_layout_vertical_mixed_ltr(text)?.inline_advance)
    }

    /// Measures exactly [`Self::shape_vertical_mixed_ltr`], including both
    /// components of every positioned origin and advance-vector endpoint.
    pub fn measure_layout_vertical_mixed_ltr(&self, text: &str) -> Result<LayoutMetrics, Error> {
        Ok(measure_vertical_run(&self.shape_vertical_mixed_ltr(text)?))
    }

    /// Exports the final mixed run with the normal SVG ink padding.
    pub fn render_svg_vertical_mixed_ltr(&self, text: &str) -> Result<String, Error> {
        glyph_run_to_svg(&self.shape_vertical_mixed_ltr(text)?, &self.svg_unit)
    }

    /// Shapes text into a [`GlyphRun`].
    pub fn text2glyph_run(&self, text: &str) -> Result<GlyphRun, Error> {
        let mut options = self.options();
        options.font = Some(crate::FontRef::Loaded(self.face));
        crate::commands::text2commands(text, options)
    }

    /// Alias for [`FontEngine::text2glyph_run`].
    pub fn text2commands(&self, text: &str) -> Result<GlyphRun, Error> {
        self.text2glyph_run(text)
    }

    /// Measures the inline extent of shaped text.
    pub fn measure(&self, text: &str) -> Result<f64, Error> {
        Ok(self.measure_layout(text)?.inline_advance)
    }

    /// Measures logical and ink extents from the same run returned by [`Self::shape`].
    pub fn measure_layout(&self, text: &str) -> Result<LayoutMetrics, Error> {
        let run = self.text2glyph_run(text)?;
        let direction = self.options().text_direction;
        let inline_advance: f32 = match direction {
            TextDirection::TopToBottom => run
                .glyphs
                .iter()
                .map(|glyph| glyph.glyph.metrics.advance_y)
                .sum(),
            TextDirection::RightToLeft => run
                .glyphs
                .iter()
                .map(|glyph| glyph.glyph.metrics.advance_x)
                .sum::<f32>()
                .abs(),
            TextDirection::LeftToRight => run
                .glyphs
                .iter()
                .map(|glyph| glyph.glyph.metrics.advance_x)
                .sum(),
        };
        let mut ink_bounds = None;
        let mut line_bounds = None;
        for positioned in &run.glyphs {
            if let Some(bounds) = positioned.glyph.metrics.bounds {
                extend_bounds(
                    &mut ink_bounds,
                    positioned.x + bounds.min_x,
                    positioned.y + bounds.min_y,
                );
                extend_bounds(
                    &mut ink_bounds,
                    positioned.x + bounds.max_x,
                    positioned.y + bounds.max_y,
                );
            }
            let (end_x, end_y) = if direction.is_vertical() {
                (
                    positioned.x,
                    positioned.y + positioned.glyph.metrics.advance_y,
                )
            } else {
                (
                    positioned.x + positioned.glyph.metrics.advance_x,
                    positioned.y,
                )
            };
            extend_bounds(&mut line_bounds, positioned.x, positioned.y);
            extend_bounds(&mut line_bounds, end_x, end_y);
        }
        let block_extent = line_bounds
            .map(|bounds| {
                if direction.is_vertical() {
                    (bounds.max_x - bounds.min_x).abs()
                } else {
                    (bounds.max_y - bounds.min_y).abs()
                }
            })
            .unwrap_or(0.0);
        Ok(LayoutMetrics {
            inline_advance: inline_advance.abs() as f64,
            block_extent: block_extent as f64,
            ink_bounds,
            line_bounds,
        })
    }

    /// Renders shaped text to SVG.
    pub fn render_svg(&self, text: &str) -> Result<String, Error> {
        self.text2svg(text)
    }

    /// Renders text to SVG using vertical flow regardless of the current policy.
    pub fn render_svg_vertical(&self, text: &str) -> Result<String, Error> {
        self.clone().with_vertical_flow().render_svg(text)
    }

    /// Alias for [`FontEngine::render_svg`].
    pub fn text2svg(&self, text: &str) -> Result<String, Error> {
        let run = self.text2glyph_run(text)?;
        glyph_run_to_svg(&run, &self.svg_unit)
    }
}

fn validate_single_line(text: &str, context: &str) -> Result<(), Error> {
    if text.chars().any(|ch| {
        matches!(
            ch,
            '\t' | '\r' | '\n' | '\u{000b}' | '\u{000c}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
        )
    }) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("{context} requires one line without tabs or line-break controls"),
        ));
    }
    Ok(())
}

fn mixed_orientation_groups(text: &str) -> Vec<(&str, bool)> {
    let mut groups = Vec::new();
    let (mut start, mut end) = (0, 0);
    let mut previous = None;
    for cluster in crate::unicode::graphemes::extended_graphemes(text) {
        let sideways = cluster.chars().next().is_some_and(|first| {
            crate::unicode::vertical_orientation(first)
                == crate::unicode::VerticalOrientation::Rotated
        }) && !cluster.chars().any(crate::unicode::is_enclosing_mark);
        if let Some(orientation) = previous {
            if orientation != sideways {
                groups.push((&text[start..end], orientation));
                start = end;
            }
        }
        end += cluster.len();
        previous = Some(sideways);
    }
    if let Some(sideways) = previous {
        groups.push((&text[start..end], sideways));
    }
    groups
}

fn measure_vertical_run(run: &GlyphRun) -> LayoutMetrics {
    let mut ink_bounds = None;
    let mut line_bounds = None;
    let mut inline_advance = 0.0f32;
    for positioned in &run.glyphs {
        let metrics = &positioned.glyph.metrics;
        inline_advance += metrics.advance_y;
        if let Some(bounds) = metrics.bounds {
            extend_bounds(
                &mut ink_bounds,
                positioned.x + bounds.min_x,
                positioned.y + bounds.min_y,
            );
            extend_bounds(
                &mut ink_bounds,
                positioned.x + bounds.max_x,
                positioned.y + bounds.max_y,
            );
        }
        extend_bounds(&mut line_bounds, positioned.x, positioned.y);
        extend_bounds(
            &mut line_bounds,
            positioned.x + metrics.advance_x,
            positioned.y + metrics.advance_y,
        );
    }
    LayoutMetrics {
        inline_advance: inline_advance.abs() as f64,
        block_extent: line_bounds
            .map(|bounds| (bounds.max_x - bounds.min_x).abs() as f64)
            .unwrap_or(0.0),
        ink_bounds,
        line_bounds,
    }
}

fn validate_outline_run(run: &GlyphRun, context: &str) -> Result<(), Error> {
    // Validate the entire run before changing any geometry. Capability-based
    // scope also permits pathified SVG or foreground COLR that meets it.
    for positioned in &run.glyphs {
        for layer in &positioned.glyph.layers {
            if !matches!(layer, GlyphLayer::Path(path)
                if path.paint == GlyphPaint::CurrentColor
                    && path.paint_mode == PathPaintMode::Fill
                    && path.clip_commands.is_empty())
            {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    format!("{context} supports only current-color, clip-free fill paths"),
                ));
            }
        }
    }
    Ok(())
}

fn rotate_sideways_ltr(mut run: GlyphRun) -> Result<GlyphRun, Error> {
    validate_outline_run(&run, "sideways LTR")?;
    for positioned in &mut run.glyphs {
        (positioned.x, positioned.y) = (-positioned.y, positioned.x);
        let glyph = &mut positioned.glyph;
        let metrics = &mut glyph.metrics;
        (metrics.advance_x, metrics.advance_y) = (-metrics.advance_y, metrics.advance_x);
        // These legacy fields describe an LSB/positive-up ascender reference,
        // not a y-down point. Their y-down point is (bearing_x, -bearing_y).
        (metrics.bearing_x, metrics.bearing_y) = (metrics.bearing_y, -metrics.bearing_x);
        metrics.bounds = metrics.bounds.map(|bounds| GlyphBounds {
            min_x: -bounds.max_y,
            min_y: bounds.min_x,
            max_x: -bounds.min_y,
            max_y: bounds.max_x,
        });
        if let Some(font) = glyph.font.as_mut() {
            font.flow = crate::commands::GlyphFlow::Vertical;
        }
        for layer in &mut glyph.layers {
            if let GlyphLayer::Path(path) = layer {
                (path.offset_x, path.offset_y) = (-path.offset_y, path.offset_x);
                for command in &mut path.commands {
                    *command = match *command {
                        Command::MoveTo(x, y) => Command::MoveTo(-y, x),
                        Command::Line(x, y) => Command::Line(-y, x),
                        Command::Bezier((cx, cy), (x, y)) => Command::Bezier((-cy, cx), (-y, x)),
                        Command::CubicBezier((ax, ay), (bx, by), (x, y)) => {
                            Command::CubicBezier((-ay, ax), (-by, bx), (-y, x))
                        }
                        Command::Close => Command::Close,
                    };
                }
            }
        }
    }
    Ok(run)
}

fn extend_bounds(bounds: &mut Option<GlyphBounds>, x: f32, y: f32) {
    if let Some(bounds) = bounds.as_mut() {
        bounds.min_x = bounds.min_x.min(x);
        bounds.min_y = bounds.min_y.min(y);
        bounds.max_x = bounds.max_x.max(x);
        bounds.max_y = bounds.max_y.max(y);
    } else {
        *bounds = Some(GlyphBounds {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_svg_is_exactly_the_final_run_export() {
        let file = crate::FontFile::from_buffer(include_bytes!(
            "../tests/fixtures/mixed-vertical-ltr.ttc"
        ))
        .unwrap();
        let face = file.face(1).unwrap();
        let engine = face.engine().with_font_size(100.0).with_svg_unit("pt");
        for text in ["", " ", "漢AV漢", "A\u{302a}「", "1\u{fe0f}\u{20e3}AV"] {
            let run = engine.shape_vertical_mixed_ltr(text).unwrap();
            assert_eq!(
                engine.render_svg_vertical_mixed_ltr(text).unwrap(),
                glyph_run_to_svg(&run, "pt").unwrap()
            );
            assert_eq!(
                engine.measure_layout_vertical_mixed_ltr(text).unwrap(),
                measure_vertical_run(&run)
            );
        }
    }

    #[test]
    fn shaping_policy_maps_to_text_direction() {
        assert_eq!(
            ShapingPolicy::LeftToRight.text_direction(),
            TextDirection::LeftToRight
        );
        assert_eq!(
            ShapingPolicy::RightToLeft.text_direction(),
            TextDirection::RightToLeft
        );
        assert_eq!(
            ShapingPolicy::TopToBottom.text_direction(),
            TextDirection::TopToBottom
        );
    }

    fn path_run(layer: GlyphLayer) -> GlyphRun {
        GlyphRun::new(vec![
            PositionedGlyph::new(
                crate::Glyph::new(vec![GlyphLayer::Path(crate::PathGlyphLayer::new(
                    vec![Command::MoveTo(1.0, 2.0)],
                    GlyphPaint::CurrentColor,
                ))]),
                3.0,
                4.0,
            ),
            PositionedGlyph::new(crate::Glyph::new(vec![layer]), 5.0, 6.0),
        ])
    }

    #[test]
    fn sideways_rotation_preserves_offsets_and_all_path_control_points() {
        let mut path = crate::PathGlyphLayer::new(
            vec![
                Command::MoveTo(1.0, 2.0),
                Command::Line(3.0, 4.0),
                Command::Bezier((5.0, 6.0), (7.0, 8.0)),
                Command::CubicBezier((9.0, 10.0), (11.0, 12.0), (13.0, 14.0)),
                Command::Close,
            ],
            GlyphPaint::CurrentColor,
        );
        path.offset_x = 15.0;
        path.offset_y = 16.0;
        path.fill_rule = FillRule::EvenOdd;
        let run = rotate_sideways_ltr(path_run(GlyphLayer::Path(path))).unwrap();
        let glyph = &run.glyphs[1];
        assert_eq!((glyph.x, glyph.y), (-6.0, 5.0));
        let GlyphLayer::Path(path) = &glyph.glyph.layers[0] else {
            panic!("expected path")
        };
        assert_eq!((path.offset_x, path.offset_y), (-16.0, 15.0));
        assert_eq!(path.fill_rule, FillRule::EvenOdd);
        assert_eq!(
            format!("{:?}", path.commands),
            "[MoveTo(-2.0, 1.0), Line(-4.0, 3.0), Bezier((-6.0, 5.0), (-8.0, 7.0)), CubicBezier((-10.0, 9.0), (-12.0, 11.0), (-14.0, 13.0)), Close]"
        );
    }

    #[test]
    fn sideways_rejects_every_unsupported_layer_instead_of_partial_rotation() {
        let path =
            crate::PathGlyphLayer::new(vec![Command::MoveTo(1.0, 2.0)], GlyphPaint::CurrentColor);
        let mut unsupported = Vec::new();
        let mut clipped = path.clone();
        clipped.clip_commands.push(Command::Close);
        unsupported.push(GlyphLayer::Path(clipped));
        let mut stroke = path.clone();
        stroke.paint_mode = PathPaintMode::Stroke;
        unsupported.push(GlyphLayer::Path(stroke));
        for paint in [
            GlyphPaint::Solid(0xff12_3456),
            GlyphPaint::LinearGradient(GlyphLinearGradient {
                x1: 0.0,
                y1: 0.0,
                x2: 1.0,
                y2: 1.0,
                units: GlyphGradientUnits::UserSpaceOnUse,
                transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                spread: GlyphGradientSpread::Pad,
                stops: vec![],
            }),
            GlyphPaint::RadialGradient(GlyphRadialGradient {
                cx: 0.0,
                cy: 0.0,
                r: 1.0,
                fx: 0.0,
                fy: 0.0,
                units: GlyphGradientUnits::UserSpaceOnUse,
                transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                spread: GlyphGradientSpread::Pad,
                stops: vec![],
            }),
        ] {
            let mut painted = path.clone();
            painted.paint = paint;
            unsupported.push(GlyphLayer::Path(painted));
        }
        unsupported.push(GlyphLayer::Raster(RasterGlyphLayer::from_rgba(
            1,
            1,
            vec![0; 4],
        )));
        unsupported.push(GlyphLayer::Raster(RasterGlyphLayer::from_encoded(vec![])));
        #[cfg(feature = "svg-fonts")]
        unsupported.push(GlyphLayer::Svg(crate::commands::SvgGlyphLayer {
            document: "<svg/>".to_owned(),
            view_box_min_x: 0.0,
            view_box_min_y: 0.0,
            view_box_width: 1.0,
            view_box_height: 1.0,
            width: 1.0,
            height: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }));
        for layer in unsupported {
            let run = path_run(layer);
            let original = format!("{run:?}");
            assert_eq!(
                rotate_sideways_ltr(run.clone()).unwrap_err().kind(),
                ErrorKind::Unsupported
            );
            assert_eq!(format!("{run:?}"), original);
        }
    }

    #[test]
    #[cfg(feature = "raw")]
    fn engine_stores_font_variant() {
        let face = crate::FontFile::from_buffer(include_bytes!(
            "../tests/fixtures/vertical-no-metrics.ttf"
        ))
        .expect("load font file")
        .current_face()
        .expect("current face");
        let engine = FontEngine::new(&face).with_jis78().with_vertical_flow();

        assert_eq!(engine.font_variant(), FontVariant::Jis78);
        assert_eq!(engine.shaping_policy(), ShapingPolicy::TopToBottom);
        assert_eq!(engine.options().font_variant, FontVariant::Jis78);
        assert_eq!(engine.options().text_direction, TextDirection::TopToBottom);
    }

    #[test]
    fn glyph_run_to_svg_writes_stroke_path_attributes() {
        let run = GlyphRun::new(vec![PositionedGlyph::new(
            crate::Glyph::new(vec![GlyphLayer::Path(crate::PathGlyphLayer::stroke(
                vec![Command::MoveTo(1.0, 2.0), Command::Line(3.0, 4.0)],
                GlyphPaint::Solid(0xff11_2233),
                2.5,
            ))]),
            0.0,
            0.0,
        )]);

        let svg = glyph_run_to_svg(&run, "px").expect("svg export");

        assert!(svg.contains("fill=\"none\""));
        assert!(svg.contains("stroke=\"#112233\""));
        assert!(svg.contains("stroke-width=\"2.5\""));
    }

    #[test]
    fn glyph_run_to_svg_writes_gradient_defs() {
        let run = GlyphRun::new(vec![PositionedGlyph::new(
            crate::Glyph::new(vec![GlyphLayer::Path(crate::PathGlyphLayer::new(
                vec![
                    Command::MoveTo(0.0, 0.0),
                    Command::Line(10.0, 0.0),
                    Command::Close,
                ],
                GlyphPaint::LinearGradient(crate::GlyphLinearGradient {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 10.0,
                    y2: 0.0,
                    units: crate::GlyphGradientUnits::UserSpaceOnUse,
                    transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                    spread: crate::GlyphGradientSpread::Pad,
                    stops: vec![
                        crate::GlyphGradientStop {
                            offset: 0.0,
                            color: 0xff11_2233,
                        },
                        crate::GlyphGradientStop {
                            offset: 1.0,
                            color: 0x8011_2233,
                        },
                    ],
                }),
            ))]),
            0.0,
            0.0,
        )]);

        let svg = glyph_run_to_svg(&run, "px").expect("svg export");

        assert!(svg.contains("<defs><linearGradient"));
        assert!(svg.contains("fill=\"url(#glyph-gradient-0)\""));
        assert!(svg.contains("gradientUnits=\"userSpaceOnUse\""));
        assert!(svg.contains("gradientTransform=\"matrix(1 0 0 1 0 0)\""));
        assert!(svg.contains("stop-opacity=\""));
    }

    #[test]
    fn glyph_run_to_svg_writes_clip_path_defs() {
        let mut layer = crate::PathGlyphLayer::new(
            vec![
                Command::MoveTo(0.0, 0.0),
                Command::Line(10.0, 0.0),
                Command::Close,
            ],
            GlyphPaint::Solid(0xff11_2233),
        );
        layer.clip_commands = vec![
            Command::MoveTo(1.0, 2.0),
            Command::Line(3.0, 2.0),
            Command::Line(3.0, 4.0),
            Command::Close,
        ];
        let run = GlyphRun::new(vec![PositionedGlyph::new(
            crate::Glyph::new(vec![GlyphLayer::Path(layer)]),
            0.0,
            0.0,
        )]);

        let svg = glyph_run_to_svg(&run, "px").expect("svg export");

        assert!(svg.contains("<clipPath id=\"glyph-clip-"));
        assert!(svg.contains("clip-path=\"url(#glyph-clip-"));
    }
}

pub(crate) fn glyph_run_to_svg(run: &GlyphRun, fontunit: &str) -> Result<String, Error> {
    let Some(bounds) = glyph_run_bounds(run)? else {
        let size = format!("0{}", fontunit);
        return Ok(format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 0 0\"></svg>",
            size, size
        ));
    };

    const SVG_EXPORT_PADDING: f32 = 4.0;
    let min_x = bounds.min_x - SVG_EXPORT_PADDING;
    let min_y = bounds.min_y - SVG_EXPORT_PADDING;
    let view_width = (bounds.max_x - bounds.min_x + SVG_EXPORT_PADDING * 2.0).max(1.0);
    let view_height = (bounds.max_y - bounds.min_y + SVG_EXPORT_PADDING * 2.0).max(1.0);
    let width = view_width.ceil();
    let height = view_height.ceil();
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}{}\" height=\"{}{}\" viewBox=\"{} {} {} {}\" overflow=\"visible\">",
        width, fontunit, height, fontunit, min_x, min_y, view_width, view_height
    );
    let mut registry = SvgDefRegistry::default();
    let mut body = String::new();

    for glyph in &run.glyphs {
        for layer in &glyph.glyph.layers {
            match layer {
                GlyphLayer::Path(path) => {
                    let d = draw_commands_to_svg_path(
                        &path.commands,
                        glyph.x + path.offset_x,
                        glyph.y + path.offset_y,
                    );
                    if d.is_empty() {
                        continue;
                    }
                    body += &format!(
                        "<path d=\"{}\" {} />",
                        d,
                        path_to_svg_attributes(path, &mut registry)
                    );
                }
                GlyphLayer::Raster(raster) => {
                    body += &raster_layer_to_svg_image(raster, glyph.x, glyph.y)?;
                }
                #[cfg(feature = "svg-fonts")]
                GlyphLayer::Svg(layer) => {
                    let _ = (layer, glyph);
                    return Err(Error::new(
                        ErrorKind::Unsupported,
                        "unsafe or unsupported embedded SVG glyph layer",
                    ));
                }
            }
        }
    }

    if !registry.defs.is_empty() {
        svg += "<defs>";
        for definition in &registry.defs {
            svg += definition;
        }
        svg += "</defs>";
    }
    svg += &body;
    svg += "</svg>";
    Ok(svg)
}

fn glyph_run_bounds(run: &GlyphRun) -> Result<Option<GlyphBounds>, Error> {
    let mut bounds = None;

    for glyph in &run.glyphs {
        let glyph_bounds = if let Some(bounds) = glyph.glyph.metrics.bounds {
            Some(GlyphBounds {
                min_x: bounds.min_x + glyph.x,
                min_y: bounds.min_y + glyph.y,
                max_x: bounds.max_x + glyph.x,
                max_y: bounds.max_y + glyph.y,
            })
        } else {
            glyph_layer_bounds(glyph)?
        };

        if let Some(glyph_bounds) = glyph_bounds {
            extend_glyph_bounds(&mut bounds, glyph_bounds);
        }
    }

    Ok(bounds)
}

fn glyph_layer_bounds(glyph: &PositionedGlyph) -> Result<Option<GlyphBounds>, Error> {
    let mut bounds = None;

    for layer in &glyph.glyph.layers {
        match layer {
            GlyphLayer::Path(path) => {
                let stroke_padding = if matches!(path.paint_mode, PathPaintMode::Stroke) {
                    path.stroke_width / 2.0
                } else {
                    0.0
                };
                for command in &path.commands {
                    match command {
                        Command::MoveTo(x, y) | Command::Line(x, y) => {
                            extend_path_point(
                                &mut bounds,
                                glyph.x + path.offset_x + *x,
                                glyph.y + path.offset_y + *y,
                                stroke_padding,
                            );
                        }
                        Command::Bezier((cx, cy), (x, y)) => {
                            extend_path_point(
                                &mut bounds,
                                glyph.x + path.offset_x + *cx,
                                glyph.y + path.offset_y + *cy,
                                stroke_padding,
                            );
                            extend_path_point(
                                &mut bounds,
                                glyph.x + path.offset_x + *x,
                                glyph.y + path.offset_y + *y,
                                stroke_padding,
                            );
                        }
                        Command::CubicBezier((xa, ya), (xb, yb), (xc, yc)) => {
                            extend_path_point(
                                &mut bounds,
                                glyph.x + path.offset_x + *xa,
                                glyph.y + path.offset_y + *ya,
                                stroke_padding,
                            );
                            extend_path_point(
                                &mut bounds,
                                glyph.x + path.offset_x + *xb,
                                glyph.y + path.offset_y + *yb,
                                stroke_padding,
                            );
                            extend_path_point(
                                &mut bounds,
                                glyph.x + path.offset_x + *xc,
                                glyph.y + path.offset_y + *yc,
                                stroke_padding,
                            );
                        }
                        Command::Close => {}
                    }
                }
            }
            GlyphLayer::Raster(raster) => {
                if let Some((width, height)) = raster_layer_dimensions(raster)? {
                    extend_point(
                        &mut bounds,
                        glyph.x + raster.offset_x,
                        glyph.y + raster.offset_y,
                    );
                    extend_point(
                        &mut bounds,
                        glyph.x + raster.offset_x + width,
                        glyph.y + raster.offset_y + height,
                    );
                }
            }
            #[cfg(feature = "svg-fonts")]
            GlyphLayer::Svg(layer) => {
                extend_point(
                    &mut bounds,
                    glyph.x + layer.offset_x + layer.view_box_min_x,
                    glyph.y + layer.offset_y + layer.view_box_min_y,
                );
                extend_point(
                    &mut bounds,
                    glyph.x + layer.offset_x + layer.view_box_min_x + layer.view_box_width,
                    glyph.y + layer.offset_y + layer.view_box_min_y + layer.view_box_height,
                );
            }
        }
    }

    Ok(bounds)
}

fn extend_glyph_bounds(bounds: &mut Option<GlyphBounds>, next: GlyphBounds) {
    if let Some(bounds) = bounds.as_mut() {
        bounds.min_x = bounds.min_x.min(next.min_x);
        bounds.min_y = bounds.min_y.min(next.min_y);
        bounds.max_x = bounds.max_x.max(next.max_x);
        bounds.max_y = bounds.max_y.max(next.max_y);
    } else {
        *bounds = Some(next);
    }
}

fn extend_point(bounds: &mut Option<GlyphBounds>, x: f32, y: f32) {
    if let Some(bounds) = bounds.as_mut() {
        bounds.min_x = bounds.min_x.min(x);
        bounds.min_y = bounds.min_y.min(y);
        bounds.max_x = bounds.max_x.max(x);
        bounds.max_y = bounds.max_y.max(y);
    } else {
        *bounds = Some(GlyphBounds {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        });
    }
}

fn extend_path_point(bounds: &mut Option<GlyphBounds>, x: f32, y: f32, stroke_padding: f32) {
    extend_point(bounds, x - stroke_padding, y - stroke_padding);
    if stroke_padding > 0.0 {
        extend_point(bounds, x + stroke_padding, y + stroke_padding);
    }
}

fn draw_commands_to_svg_path(commands: &[Command], origin_x: f32, origin_y: f32) -> String {
    let mut d = String::new();
    for command in commands {
        match command {
            Command::MoveTo(x, y) => d += &format!("M{} {} ", origin_x + *x, origin_y + *y),
            Command::Line(x, y) => d += &format!("L{} {} ", origin_x + *x, origin_y + *y),
            Command::Bezier((cx, cy), (x, y)) => {
                d += &format!(
                    "Q{} {} {} {} ",
                    origin_x + *cx,
                    origin_y + *cy,
                    origin_x + *x,
                    origin_y + *y
                )
            }
            Command::CubicBezier((xa, ya), (xb, yb), (xc, yc)) => {
                d += &format!(
                    "C{} {} {} {} {} {} ",
                    origin_x + *xa,
                    origin_y + *ya,
                    origin_x + *xb,
                    origin_y + *yb,
                    origin_x + *xc,
                    origin_y + *yc
                )
            }
            Command::Close => d += "Z ",
        }
    }
    d.trim_end().to_string()
}

fn normalize_svg_color(color: u32) -> u32 {
    if color <= 0x00ff_ffff {
        0xff00_0000 | color
    } else {
        color
    }
}

fn paint_to_svg_attributes(
    paint: &GlyphPaint,
    registry: &mut SvgDefRegistry,
    attribute: &str,
) -> String {
    match paint {
        GlyphPaint::CurrentColor => format!("{attribute}=\"currentColor\""),
        GlyphPaint::Solid(color) => {
            let color = normalize_svg_color(*color);
            let alpha = ((color >> 24) & 0xff) as u8;
            let red = ((color >> 16) & 0xff) as u8;
            let green = ((color >> 8) & 0xff) as u8;
            let blue = (color & 0xff) as u8;
            if alpha == 0xff {
                format!("{attribute}=\"#{:02x}{:02x}{:02x}\"", red, green, blue)
            } else {
                format!(
                    "{attribute}=\"#{:02x}{:02x}{:02x}\" {attribute}-opacity=\"{}\"",
                    red,
                    green,
                    blue,
                    alpha as f32 / 255.0
                )
            }
        }
        GlyphPaint::LinearGradient(gradient) => {
            let id = registry.register_linear_gradient(gradient);
            format!("{attribute}=\"url(#{id})\"")
        }
        GlyphPaint::RadialGradient(gradient) => {
            let id = registry.register_radial_gradient(gradient);
            format!("{attribute}=\"url(#{id})\"")
        }
    }
}

fn path_to_svg_attributes(
    path: &crate::commands::PathGlyphLayer,
    registry: &mut SvgDefRegistry,
) -> String {
    let clip = if path.clip_commands.is_empty() {
        String::new()
    } else {
        let id = registry.register_clip_path(&path.clip_commands);
        format!(" clip-path=\"url(#{id})\"")
    };
    match path.paint_mode {
        PathPaintMode::Fill => format!(
            "{}{}{}",
            paint_to_svg_attributes(&path.paint, registry, "fill"),
            fill_rule_to_svg_attribute(path.fill_rule),
            clip
        ),
        PathPaintMode::Stroke => format!(
            "fill=\"none\" {} stroke-width=\"{}\"{}",
            paint_to_svg_attributes(&path.paint, registry, "stroke"),
            path.stroke_width,
            clip
        ),
    }
}

#[derive(Default)]
struct SvgDefRegistry {
    defs: Vec<String>,
    next_id: usize,
}

impl SvgDefRegistry {
    fn register_linear_gradient(&mut self, gradient: &GlyphLinearGradient) -> String {
        let id = format!("glyph-gradient-{}", self.next_id);
        self.next_id += 1;
        self.defs.push(linear_gradient_to_svg_def(&id, gradient));
        id
    }

    fn register_radial_gradient(&mut self, gradient: &GlyphRadialGradient) -> String {
        let id = format!("glyph-gradient-{}", self.next_id);
        self.next_id += 1;
        self.defs.push(radial_gradient_to_svg_def(&id, gradient));
        id
    }

    fn register_clip_path(&mut self, commands: &[Command]) -> String {
        let id = format!("glyph-clip-{}", self.next_id);
        self.next_id += 1;
        self.defs.push(clip_path_to_svg_def(&id, commands));
        id
    }
}

fn linear_gradient_to_svg_def(id: &str, gradient: &GlyphLinearGradient) -> String {
    let mut definition = format!(
        "<linearGradient id=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" gradientUnits=\"{}\" gradientTransform=\"{}\" spreadMethod=\"{}\">",
        id,
        gradient.x1,
        gradient.y1,
        gradient.x2,
        gradient.y2,
        gradient_units_to_svg(gradient.units),
        gradient_transform_to_svg(&gradient.transform),
        gradient_spread_to_svg(gradient.spread)
    );
    append_gradient_stops(&mut definition, &gradient.stops);
    definition += "</linearGradient>";
    definition
}

fn radial_gradient_to_svg_def(id: &str, gradient: &GlyphRadialGradient) -> String {
    let mut definition = format!(
        "<radialGradient id=\"{}\" cx=\"{}\" cy=\"{}\" r=\"{}\" fx=\"{}\" fy=\"{}\" gradientUnits=\"{}\" gradientTransform=\"{}\" spreadMethod=\"{}\">",
        id,
        gradient.cx,
        gradient.cy,
        gradient.r,
        gradient.fx,
        gradient.fy,
        gradient_units_to_svg(gradient.units),
        gradient_transform_to_svg(&gradient.transform),
        gradient_spread_to_svg(gradient.spread)
    );
    append_gradient_stops(&mut definition, &gradient.stops);
    definition += "</radialGradient>";
    definition
}

fn clip_path_to_svg_def(id: &str, commands: &[Command]) -> String {
    format!(
        "<clipPath id=\"{}\"><path d=\"{}\"/></clipPath>",
        id,
        draw_commands_to_svg_path(commands, 0.0, 0.0)
    )
}

fn append_gradient_stops(definition: &mut String, stops: &[crate::GlyphGradientStop]) {
    for stop in stops {
        let color = normalize_svg_color(stop.color);
        let alpha = ((color >> 24) & 0xff) as u8;
        let red = ((color >> 16) & 0xff) as u8;
        let green = ((color >> 8) & 0xff) as u8;
        let blue = (color & 0xff) as u8;
        let _ = if alpha == 0xff {
            write!(
                definition,
                "<stop offset=\"{}%\" stop-color=\"#{:02x}{:02x}{:02x}\"/>",
                stop.offset * 100.0,
                red,
                green,
                blue
            )
        } else {
            write!(
                definition,
                "<stop offset=\"{}%\" stop-color=\"#{:02x}{:02x}{:02x}\" stop-opacity=\"{}\"/>",
                stop.offset * 100.0,
                red,
                green,
                blue,
                alpha as f32 / 255.0
            )
        };
    }
}

fn gradient_spread_to_svg(spread: GlyphGradientSpread) -> &'static str {
    match spread {
        GlyphGradientSpread::Pad => "pad",
        GlyphGradientSpread::Reflect => "reflect",
        GlyphGradientSpread::Repeat => "repeat",
    }
}

fn gradient_units_to_svg(units: GlyphGradientUnits) -> &'static str {
    match units {
        GlyphGradientUnits::ObjectBoundingBox => "objectBoundingBox",
        GlyphGradientUnits::UserSpaceOnUse => "userSpaceOnUse",
    }
}

fn gradient_transform_to_svg(transform: &[f32; 6]) -> String {
    format!(
        "matrix({} {} {} {} {} {})",
        transform[0], transform[1], transform[2], transform[3], transform[4], transform[5]
    )
}

fn fill_rule_to_svg_attribute(fill_rule: FillRule) -> &'static str {
    match fill_rule {
        FillRule::NonZero => "",
        FillRule::EvenOdd => " fill-rule=\"evenodd\"",
    }
}

fn raster_layer_to_svg_image(
    raster: &RasterGlyphLayer,
    glyph_x: f32,
    glyph_y: f32,
) -> Result<String, Error> {
    let Some((width, height)) = raster_layer_dimensions(raster)? else {
        return Ok(String::new());
    };

    match &raster.source {
        RasterGlyphSource::Encoded(data) => {
            let Some((mime, _, _)) = util::sniff_encoded_image_dimensions(data) else {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    "encoded raster glyph format is not supported for SVG export",
                ));
            };
            let encoded = general_purpose::STANDARD.encode(data);
            Ok(format!(
                "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" href=\"data:{};base64,{}\"/>",
                glyph_x + raster.offset_x,
                glyph_y + raster.offset_y,
                width,
                height,
                mime,
                encoded
            ))
        }
        RasterGlyphSource::Rgba { .. } => Err(Error::new(
            ErrorKind::Unsupported,
            "RGBA raster glyph layers are not supported for SVG export yet",
        )),
    }
}

fn raster_layer_dimensions(raster: &RasterGlyphLayer) -> Result<Option<(f32, f32)>, Error> {
    let (source_width, source_height) = match &raster.source {
        RasterGlyphSource::Encoded(data) => {
            let Some((_, width, height)) = util::sniff_encoded_image_dimensions(data) else {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    "encoded raster glyph format is not supported for SVG export",
                ));
            };
            (width, height)
        }
        RasterGlyphSource::Rgba { width, height, .. } => (*width, *height),
    };

    let width = raster.width.unwrap_or(source_width);
    let height = raster.height.unwrap_or(source_height);
    if width == 0 || height == 0 {
        return Ok(None);
    }

    Ok(Some((width as f32, height as f32)))
}
