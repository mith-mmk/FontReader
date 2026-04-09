//! High-level shaping and rendering engine bound to one [`crate::FontFace`].

#[cfg(feature = "svg-fonts")]
use crate::commands::SvgGlyphLayer;
use crate::commands::{
    Command, FillRule, FontOptions, FontVariant, FontVariationSetting, GlyphBounds, GlyphLayer,
    GlyphPaint, GlyphRun, PathPaintMode, PositionedGlyph, RasterGlyphLayer, RasterGlyphSource,
    TextDirection,
};
use crate::fontface::FontFace;
use crate::util;
use base64::{engine::general_purpose, Engine as _};
use std::io::{Error, ErrorKind};

/// Public shaping direction used by [`FontEngine`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapingPolicy {
    LeftToRight,
    RightToLeft,
    TopToBottom,
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
        let mut options = self.options();
        options.font = Some(crate::FontRef::Loaded(self.face));
        self.face.font().measure_with_options(text, &options)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "raw")]
    fn japanese_font_path() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".test_fonts")
            .join("NotoSansJP-Regular.otf")
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

    #[test]
    #[cfg(feature = "raw")]
    fn engine_stores_font_variant() {
        let face = crate::FontFile::from_file(japanese_font_path())
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
                    svg += &format!("<path d=\"{}\" {} />", d, path_to_svg_attributes(path));
                }
                GlyphLayer::Raster(raster) => {
                    svg += &raster_layer_to_svg_image(raster, glyph.x, glyph.y)?;
                }
                #[cfg(feature = "svg-fonts")]
                GlyphLayer::Svg(layer) => {
                    svg += &svg_layer_to_svg_fragment(layer, glyph.x, glyph.y);
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

fn path_to_svg_attributes(path: &crate::commands::PathGlyphLayer) -> String {
    let paint = paint_to_svg_attributes(path.paint);
    match path.paint_mode {
        PathPaintMode::Fill => format!("{}{}", paint, fill_rule_to_svg_attribute(path.fill_rule)),
        PathPaintMode::Stroke => format!(
            "fill=\"none\" {} stroke-width=\"{}\"",
            paint.replacen("fill=", "stroke=", 1)
                .replacen(" fill-opacity=", " stroke-opacity=", 1),
            path.stroke_width
        ),
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

#[cfg(feature = "svg-fonts")]
fn svg_layer_to_svg_fragment(layer: &SvgGlyphLayer, glyph_x: f32, glyph_y: f32) -> String {
    format!(
        "<svg x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\" overflow=\"visible\">{}</svg>",
        glyph_x + layer.offset_x,
        glyph_y + layer.offset_y,
        layer.width,
        layer.height,
        layer.view_box_min_x,
        layer.view_box_min_y,
        layer.view_box_width,
        layer.view_box_height,
        layer.document
    )
}
