//! Explicit, atomic horizontal-in-vertical outline composition.

use super::{extend_bounds, glyph_run_to_svg, validate_outline_run, validate_single_line};
use super::{FontEngine, LayoutMetrics};
use crate::commands::{
    Command, FontMetrics, Glyph, GlyphBounds, GlyphFlow, GlyphLayer, GlyphMetrics, GlyphRun,
    PositionedGlyph,
};
use std::io::{Error, ErrorKind};

impl FontEngine<'_> {
    /// Composes one explicitly selected LTR string into a tate-chu-yoko cell.
    ///
    /// Ordinary horizontal shaping runs first, retaining supported GSUB/GPOS,
    /// style, locale, variants, variations, size and stretch. No vertical or
    /// rotation feature is selected. The advance width is centered at x=0 and
    /// compressed horizontally only if wider than `font_size`. The baseline is
    /// `font_size / 2 + (ascent - descent) / 2`, using the scaled horizontal
    /// metrics and positive descent. Ink may overhang the cell; it is not clipped.
    ///
    /// The result is one composite glyph at (0, 0), with advance (0, font_size).
    /// Its font metrics describe the synthetic square cell (half-size ascent
    /// and descent, zero gap, vertical flow); its bearings are zero. Source
    /// glyph boundaries are not retained. An empty shaped input yields no cell.
    /// Spaces and zero-advance marks still form a cell if shaping emits glyphs.
    /// Leading/trailing spaces participate in width and centering. `line_height`
    /// does not change the cell. Bounds retain conservative outline extrema.
    ///
    /// `font_size` follows this library's sizing convention, not a guarantee of
    /// CSS em sizing. Only current-color, clip-free fill paths are supported.
    /// Tabs/line-break controls, invalid options, negative total advance and
    /// non-finite geometry are errors. Direction is overridden for this call
    /// only. This does not add automatic digit selection, width-feature
    /// substitution, BiDi, fallback, wrapping, or CSS text-combine conformance.
    pub fn shape_tate_chu_yoko_ltr(&self, text: &str) -> Result<GlyphRun, Error> {
        validate_single_line(text, "tate-chu-yoko LTR")?;
        let run = self.clone().with_left_to_right().shape(text)?;
        compose(run, self.options().font_size)
    }

    /// Returns one `font_size` advance, or zero for an empty shaped input.
    pub fn measure_tate_chu_yoko_ltr(&self, text: &str) -> Result<f64, Error> {
        Ok(self.measure_layout_tate_chu_yoko_ltr(text)?.inline_advance)
    }

    /// Measures the same composite as [`Self::shape_tate_chu_yoko_ltr`].
    ///
    /// Logical bounds are the square `[-size/2, 0, size/2, size]`, independent
    /// of ink overhangs, kerning, line height and horizontal compression.
    /// Empty shaped input has no bounds and zero extents.
    pub fn measure_layout_tate_chu_yoko_ltr(&self, text: &str) -> Result<LayoutMetrics, Error> {
        let run = self.shape_tate_chu_yoko_ltr(text)?;
        let cell = run.glyphs.first();
        let size = cell.map(|g| g.glyph.metrics.advance_y).unwrap_or(0.0);
        Ok(LayoutMetrics {
            inline_advance: size as f64,
            block_extent: size as f64,
            ink_bounds: cell.and_then(|g| g.glyph.metrics.bounds),
            line_bounds: cell.map(|_| GlyphBounds {
                min_x: -size / 2.0,
                min_y: 0.0,
                max_x: size / 2.0,
                max_y: size,
            }),
        })
    }

    /// Exports the final composite with normal SVG ink padding and SVG units.
    /// Ink-free input exports a zero-sized SVG even when it occupies a cell.
    pub fn render_svg_tate_chu_yoko_ltr(&self, text: &str) -> Result<String, Error> {
        glyph_run_to_svg(&self.shape_tate_chu_yoko_ltr(text)?, &self.svg_unit)
    }
}

fn finite(value: f64) -> Result<f32, Error> {
    let value = value as f32;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "tate-chu-yoko requires finite geometry",
        ))
    }
}

fn compose(run: GlyphRun, size: f32) -> Result<GlyphRun, Error> {
    validate_outline_run(&run, "tate-chu-yoko LTR")?;
    let Some(first) = run.glyphs.first() else {
        return Ok(run);
    };
    let width = run
        .glyphs
        .iter()
        .map(|g| g.glyph.metrics.advance_x as f64)
        .sum::<f64>();
    if !width.is_finite() || width < 0.0 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "tate-chu-yoko requires a non-negative finite horizontal advance",
        ));
    }
    let scale = if width > size as f64 {
        size as f64 / width
    } else {
        1.0
    };
    let shift_x = -width * scale / 2.0;
    let font = first.glyph.font.ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidData,
            "tate-chu-yoko requires font metrics",
        )
    })?;
    let shift_y = size as f64 / 2.0 + (font.ascent as f64 - font.descent as f64) / 2.0;
    finite(shift_y)?;
    let mut bounds = None;
    let mut layers = Vec::new();
    for positioned in run.glyphs {
        let x = positioned.x as f64;
        let y = positioned.y as f64;
        finite(x)?;
        finite(y)?;
        finite(positioned.glyph.metrics.advance_y as f64)?;
        if let Some(b) = positioned.glyph.metrics.bounds {
            extend_bounds(
                &mut bounds,
                finite((x + b.min_x as f64) * scale + shift_x)?,
                finite(y + b.min_y as f64 + shift_y)?,
            );
            extend_bounds(
                &mut bounds,
                finite((x + b.max_x as f64) * scale + shift_x)?,
                finite(y + b.max_y as f64 + shift_y)?,
            );
        }
        for mut layer in positioned.glyph.layers {
            if let GlyphLayer::Path(path) = &mut layer {
                path.offset_x = finite((x + path.offset_x as f64) * scale + shift_x)?;
                path.offset_y = finite(y + path.offset_y as f64 + shift_y)?;
                let mut point = |x: f32, y: f32| -> Result<(f32, f32), Error> {
                    let x = finite(x as f64 * scale)?;
                    let y = finite(y as f64)?;
                    // Enclose separately rounded f32 geometry, not only the
                    // ideal transform of the original conservative bounds.
                    extend_bounds(
                        &mut bounds,
                        finite(x as f64 + path.offset_x as f64)?,
                        finite(y as f64 + path.offset_y as f64)?,
                    );
                    Ok((x, y))
                };
                for command in &mut path.commands {
                    *command = match *command {
                        Command::MoveTo(x, y) => {
                            let (x, y) = point(x, y)?;
                            Command::MoveTo(x, y)
                        }
                        Command::Line(x, y) => {
                            let (x, y) = point(x, y)?;
                            Command::Line(x, y)
                        }
                        Command::Bezier((a, b), (x, y)) => {
                            Command::Bezier(point(a, b)?, point(x, y)?)
                        }
                        Command::CubicBezier((a, b), (c, d), (x, y)) => {
                            Command::CubicBezier(point(a, b)?, point(c, d)?, point(x, y)?)
                        }
                        Command::Close => Command::Close,
                    };
                }
            }
            layers.push(layer);
        }
    }
    if let Some(b) = bounds {
        // Keep the existing SVG exporter's padded f32 extents finite too.
        finite(b.max_x as f64 - b.min_x as f64 + 8.0)?;
        finite(b.max_y as f64 - b.min_y as f64 + 8.0)?;
    }
    Ok(GlyphRun::new(vec![PositionedGlyph::new(
        Glyph {
            font: Some(FontMetrics {
                ascent: size / 2.0,
                descent: size / 2.0,
                line_gap: 0.0,
                flow: GlyphFlow::Vertical,
            }),
            metrics: GlyphMetrics {
                advance_x: 0.0,
                advance_y: size,
                bounds,
                ..GlyphMetrics::default()
            },
            layers,
        },
        0.0,
        0.0,
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FillRule, GlyphPaint, PathGlyphLayer, PathPaintMode};

    fn run() -> GlyphRun {
        let mut path = PathGlyphLayer::new(
            vec![
                Command::MoveTo(-5.0, 3.0),
                Command::Bezier((1.0, 2.0), (3.0, 4.0)),
                Command::CubicBezier((5.0, 6.0), (7.0, 8.0), (9.0, 10.0)),
                Command::Close,
            ],
            GlyphPaint::CurrentColor,
        );
        path.fill_rule = FillRule::EvenOdd;
        path.offset_x = 2.0;
        path.offset_y = 3.0;
        let mut glyph = Glyph::new(vec![GlyphLayer::Path(path.clone()), GlyphLayer::Path(path)]);
        glyph.font = Some(FontMetrics {
            ascent: 8.0,
            descent: 2.0,
            line_gap: 0.0,
            flow: GlyphFlow::Horizontal,
        });
        glyph.metrics.advance_x = 20.0;
        GlyphRun::new(vec![PositionedGlyph::new(glyph, 4.0, 5.0)])
    }

    #[test]
    fn composite_preserves_layers_fill_rules_and_all_curve_points() {
        let result = compose(run(), 10.0).unwrap();
        let glyph = &result.glyphs[0].glyph;
        assert_eq!(glyph.layers.len(), 2);
        for layer in &glyph.layers {
            let GlyphLayer::Path(p) = layer else {
                panic!("path")
            };
            assert_eq!(p.fill_rule, FillRule::EvenOdd);
            assert_eq!((p.offset_x, p.offset_y), (-2.0, 16.0));
            assert_eq!(
                format!("{:?}", p.commands),
                format!(
                    "{:?}",
                    vec![
                        Command::MoveTo(-2.5, 3.0),
                        Command::Bezier((0.5, 2.0), (1.5, 4.0)),
                        Command::CubicBezier((2.5, 6.0), (3.5, 8.0), (4.5, 10.0)),
                        Command::Close,
                    ]
                )
            );
        }
    }

    #[test]
    fn composite_bounds_enclose_rounded_commands_and_offsets() {
        let mut r = run();
        let g = &mut r.glyphs[0];
        g.x = 0.0;
        g.y = 0.0;
        g.glyph.metrics.advance_x = 120.0;
        g.glyph.metrics.bounds = Some(GlyphBounds {
            min_x: 50.0,
            max_x: 50.0,
            min_y: 0.0,
            max_y: 0.0,
        });
        let mut path =
            PathGlyphLayer::new(vec![Command::MoveTo(50.0, 0.0)], GlyphPaint::CurrentColor);
        path.offset_x = 0.0;
        g.glyph.layers = vec![GlyphLayer::Path(path)];
        let result = compose(r, 100.0).unwrap();
        let glyph = &result.glyphs[0].glyph;
        let GlyphLayer::Path(path) = &glyph.layers[0] else {
            unreachable!()
        };
        let Command::MoveTo(x, y) = path.commands[0] else {
            unreachable!()
        };
        let b = glyph.metrics.bounds.unwrap();
        assert!(b.min_x <= x + path.offset_x && b.max_x >= x + path.offset_x);
        assert!(b.min_y <= y + path.offset_y && b.max_y >= y + path.offset_y);
    }

    #[test]
    fn composite_rejects_nonfinite_path_offset_position_and_advance() {
        for kind in 0..5 {
            let mut r = run();
            let g = &mut r.glyphs[0];
            match kind {
                0 => g.x = f32::NAN,
                1 => g.glyph.metrics.advance_y = f32::INFINITY,
                2 => g.glyph.font.as_mut().unwrap().ascent = f32::INFINITY,
                3 | 4 => {
                    let GlyphLayer::Path(p) = &mut g.glyph.layers[0] else {
                        unreachable!()
                    };
                    if kind == 3 {
                        p.offset_x = f32::INFINITY;
                    } else {
                        p.commands[0] = Command::MoveTo(f32::NAN, 0.0);
                    }
                }
                _ => unreachable!(),
            }
            assert_eq!(
                compose(r, 10.0).unwrap_err().kind(),
                ErrorKind::InvalidInput
            );
        }
    }

    #[test]
    fn composite_rejects_unsupported_path_attributes_without_partial_output() {
        for kind in 0..3 {
            let mut r = run();
            let GlyphLayer::Path(p) = &mut r.glyphs[0].glyph.layers[1] else {
                unreachable!()
            };
            match kind {
                0 => p.paint = GlyphPaint::Solid(0xff000000),
                1 => p.paint_mode = PathPaintMode::Stroke,
                _ => p.clip_commands.push(Command::MoveTo(0.0, 0.0)),
            }
            assert_eq!(compose(r, 10.0).unwrap_err().kind(), ErrorKind::Unsupported);
        }
    }

    #[test]
    fn svg_export_uses_exact_final_composite_and_keeps_ink_free_output_empty() {
        let file =
            crate::FontFile::from_buffer(include_bytes!("../../tests/fixtures/tate-chu-yoko.ttc"))
                .unwrap();
        let face = file.face(0).unwrap();
        let engine = face.engine().with_font_size(100.0).with_svg_unit("pt");
        for text in ["", " ", "12", "1\u{301}2"] {
            let run = engine.shape_tate_chu_yoko_ltr(text).unwrap();
            assert_eq!(
                engine.render_svg_tate_chu_yoko_ltr(text).unwrap(),
                glyph_run_to_svg(&run, "pt").unwrap()
            );
        }
        assert!(engine
            .render_svg_tate_chu_yoko_ltr(" ")
            .unwrap()
            .contains("width=\"0pt\""));
        assert_eq!(engine.measure_tate_chu_yoko_ltr(" ").unwrap(), 100.0);
    }
}
