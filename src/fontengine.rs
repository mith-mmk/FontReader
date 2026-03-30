use crate::commands::{
    Command, FillRule, FontOptions, GlyphBounds, GlyphLayer, GlyphPaint, GlyphRun,
    PositionedGlyph, RasterGlyphLayer, RasterGlyphSource,
};
use crate::fontface::FontFace;
use crate::util;
use base64::{engine::general_purpose, Engine as _};
use std::io::{Error, ErrorKind};

#[derive(Clone)]
pub struct FontEngine<'a> {
    face: &'a FontFace,
    options: FontOptions<'a>,
    svg_unit: String,
}

impl<'a> FontEngine<'a> {
    pub fn new(face: &'a FontFace) -> Self {
        Self {
            face,
            options: FontOptions::new(face),
            svg_unit: "px".to_string(),
        }
    }

    pub fn with_options(mut self, options: FontOptions<'a>) -> Self {
        self.options = options.with_face(self.face);
        self
    }

    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.options = self.options.with_font_size(font_size);
        self
    }

    pub fn with_line_height(mut self, line_height: f32) -> Self {
        self.options = self.options.with_line_height(line_height);
        self
    }

    pub fn with_locale(mut self, locale: &'a str) -> Self {
        self.options = self.options.with_locale(locale);
        self
    }

    pub fn with_svg_unit(mut self, unit: impl Into<String>) -> Self {
        self.svg_unit = unit.into();
        self
    }

    pub fn options(&self) -> FontOptions<'a> {
        self.options
    }

    pub fn shape(&self, text: &str) -> Result<GlyphRun, Error> {
        self.text2glyph_run(text)
    }

    pub fn text2glyph_run(&self, text: &str) -> Result<GlyphRun, Error> {
        let mut options = self.options;
        options.font = Some(crate::FontRef::Loaded(self.face));
        crate::commands::text2commands(text, options)
    }

    pub fn text2commands(&self, text: &str) -> Result<GlyphRun, Error> {
        self.text2glyph_run(text)
    }

    pub fn measure(&self, text: &str) -> Result<f64, Error> {
        let mut options = self.options;
        options.font = Some(crate::FontRef::Loaded(self.face));
        self.face.font().measure_with_options(text, &options)
    }

    pub fn render_svg(&self, text: &str) -> Result<String, Error> {
        self.text2svg(text)
    }

    pub fn text2svg(&self, text: &str) -> Result<String, Error> {
        let run = self.text2glyph_run(text)?;
        glyph_run_to_svg(&run, &self.svg_unit)
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
                    svg += &format!(
                        "<path d=\"{}\" {}{} />",
                        d,
                        paint_to_svg_attributes(path.paint),
                        fill_rule_to_svg_attribute(path.fill_rule),
                    );
                }
                GlyphLayer::Raster(raster) => {
                    svg += &raster_layer_to_svg_image(raster, glyph.x, glyph.y)?;
                }
            }
        }
    }

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
                for command in &path.commands {
                    match command {
                        Command::MoveTo(x, y) | Command::Line(x, y) => extend_point(
                            &mut bounds,
                            glyph.x + path.offset_x + *x,
                            glyph.y + path.offset_y + *y,
                        ),
                        Command::Bezier((cx, cy), (x, y)) => {
                            extend_point(
                                &mut bounds,
                                glyph.x + path.offset_x + *cx,
                                glyph.y + path.offset_y + *cy,
                            );
                            extend_point(
                                &mut bounds,
                                glyph.x + path.offset_x + *x,
                                glyph.y + path.offset_y + *y,
                            );
                        }
                        Command::CubicBezier((xa, ya), (xb, yb), (xc, yc)) => {
                            extend_point(
                                &mut bounds,
                                glyph.x + path.offset_x + *xa,
                                glyph.y + path.offset_y + *ya,
                            );
                            extend_point(
                                &mut bounds,
                                glyph.x + path.offset_x + *xb,
                                glyph.y + path.offset_y + *yb,
                            );
                            extend_point(
                                &mut bounds,
                                glyph.x + path.offset_x + *xc,
                                glyph.y + path.offset_y + *yc,
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

fn paint_to_svg_attributes(paint: GlyphPaint) -> String {
    match paint {
        GlyphPaint::CurrentColor => "fill=\"currentColor\"".to_string(),
        GlyphPaint::Solid(color) => {
            let color = normalize_svg_color(color);
            let alpha = ((color >> 24) & 0xff) as u8;
            let red = ((color >> 16) & 0xff) as u8;
            let green = ((color >> 8) & 0xff) as u8;
            let blue = (color & 0xff) as u8;
            if alpha == 0xff {
                format!("fill=\"#{:02x}{:02x}{:02x}\"", red, green, blue)
            } else {
                format!(
                    "fill=\"#{:02x}{:02x}{:02x}\" fill-opacity=\"{}\"",
                    red,
                    green,
                    blue,
                    alpha as f32 / 255.0
                )
            }
        }
    }
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
