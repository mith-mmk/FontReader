pub mod fontheader;
pub mod fontreader;
pub mod opentype;
pub(crate) mod util;
pub type Font = fontreader::Font;
pub use fontreader::{BitmapGlyphCommands, BitmapGlyphFormat, GlyphCommands, PathCommand};
pub mod commands;
#[deprecated(note = "use `fontloader::commands` instead")]
pub use commands as commads;
pub use commands::{
    text2commands, Command, FillRule, FontMetrics, FontOptions, FontRef, FontStretch, FontStyle,
    FontVariant, FontWeight, Glyph, GlyphBounds, GlyphFlow, GlyphLayer, GlyphMetrics, GlyphPaint,
    GlyphRun, PathGlyphLayer, PositionedGlyph, RasterGlyphLayer, RasterGlyphSource,
};
#[cfg(test)]
mod test;
pub mod woff;

use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;
use std::io::Error;
use std::io::ErrorKind;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;
#[cfg(not(target_arch = "wasm32"))]
use std::net::TcpStream;
use std::path::Path;

pub enum FontSource<'a> {
    File(&'a Path),
    Buffer(&'a [u8]),
}

#[derive(Debug, Clone)]
pub struct FontFaceDescriptor {
    pub family_name: String,
    pub font_name: Option<String>,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub font_stretch: FontStretch,
}

impl FontFaceDescriptor {
    pub fn new(family_name: impl Into<String>) -> Self {
        Self {
            family_name: family_name.into(),
            font_name: None,
            font_weight: FontWeight::default(),
            font_style: FontStyle::default(),
            font_stretch: FontStretch::default(),
        }
    }

    pub fn with_font_name(mut self, font_name: impl Into<String>) -> Self {
        self.font_name = Some(font_name.into());
        self
    }

    pub fn with_font_weight(mut self, font_weight: FontWeight) -> Self {
        self.font_weight = font_weight;
        self
    }

    pub fn with_font_style(mut self, font_style: FontStyle) -> Self {
        self.font_style = font_style;
        self
    }

    pub fn with_font_stretch(mut self, font_stretch: FontStretch) -> Self {
        self.font_stretch = font_stretch;
        self
    }

    pub fn from_loaded_font(font: &LoadedFont) -> Self {
        let font_ref = font.font();
        let family_name = font_ref.face_family_name();
        let font_name = font_ref.face_full_name();
        let font_weight = FontWeight(font_ref.face_weight_class());
        let font_style = if font_ref.face_is_italic() {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        };
        let font_stretch = FontStretch(width_class_to_stretch(font_ref.face_width_class()));

        Self {
            family_name,
            font_name,
            font_weight,
            font_style,
            font_stretch,
        }
    }
}

struct CachedFontFace {
    descriptor: FontFaceDescriptor,
    font: LoadedFont,
}

struct PendingFontFace {
    descriptor: FontFaceDescriptor,
    buffer: ChunkedFontBuffer,
}

struct FamilyLayoutResult {
    run: GlyphRun,
    max_line_width: f32,
}

pub struct FontFamily {
    name: String,
    faces: Vec<CachedFontFace>,
    pending_faces: HashMap<String, PendingFontFace>,
}

impl FontFamily {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            faces: Vec::new(),
            pending_faces: HashMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn cached_faces_len(&self) -> usize {
        self.faces.len()
    }

    pub fn pending_faces_len(&self) -> usize {
        self.pending_faces.len()
    }

    pub fn add_loaded_font(&mut self, font: LoadedFont) -> &LoadedFont {
        let descriptor = FontFaceDescriptor::from_loaded_font(&font);
        self.add_face(descriptor, font)
    }

    pub fn add_face(&mut self, descriptor: FontFaceDescriptor, font: LoadedFont) -> &LoadedFont {
        self.faces.push(CachedFontFace { descriptor, font });
        &self.faces.last().expect("face inserted").font
    }

    pub fn cached_descriptors(&self) -> Vec<&FontFaceDescriptor> {
        self.faces.iter().map(|face| &face.descriptor).collect()
    }

    /// Returns default `FontOptions` anchored to this family.
    ///
    /// The returned options resolve against this family's cache and default the requested
    /// family name to `self.name()`.
    pub fn options(&self) -> FontOptions<'_> {
        FontOptions::from_family(self).with_font_family(self.name())
    }

    pub fn begin_chunked_face(
        &mut self,
        face_id: impl Into<String>,
        descriptor: FontFaceDescriptor,
        total_size: usize,
    ) -> Result<(), Error> {
        let face_id = face_id.into();
        let buffer = ChunkedFontBuffer::new(total_size)?;
        self.pending_faces
            .insert(face_id, PendingFontFace { descriptor, buffer });
        Ok(())
    }

    pub fn append_chunk(
        &mut self,
        face_id: &str,
        offset: usize,
        bytes: &[u8],
    ) -> Result<bool, Error> {
        let pending = self.pending_faces.get_mut(face_id).ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                format!("unknown pending font face: {face_id}"),
            )
        })?;
        pending.buffer.append(offset, bytes)?;
        Ok(pending.buffer.is_complete())
    }

    pub fn missing_ranges(&self, face_id: &str) -> Result<Vec<(usize, usize)>, Error> {
        let pending = self.pending_faces.get(face_id).ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                format!("unknown pending font face: {face_id}"),
            )
        })?;
        Ok(pending.buffer.missing_ranges())
    }

    pub fn finalize_chunked_face(&mut self, face_id: &str) -> Result<&LoadedFont, Error> {
        let pending = self.pending_faces.remove(face_id).ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                format!("unknown pending font face: {face_id}"),
            )
        })?;
        let font = pending.buffer.into_loaded_font()?;
        Ok(self.add_face(pending.descriptor, font))
    }

    pub fn resolve_loaded_font(
        &self,
        family_name: Option<&str>,
        font_name: Option<&str>,
        font_weight: FontWeight,
        font_style: FontStyle,
        font_stretch: FontStretch,
    ) -> Option<&LoadedFont> {
        self.find_best_face(
            family_name,
            font_name,
            font_weight,
            font_style,
            font_stretch,
        )
        .map(|face| &face.font)
    }

    pub fn resolve_descriptor(
        &self,
        family_name: Option<&str>,
        font_name: Option<&str>,
        font_weight: FontWeight,
        font_style: FontStyle,
        font_stretch: FontStretch,
    ) -> Option<&FontFaceDescriptor> {
        self.find_best_face(
            family_name,
            font_name,
            font_weight,
            font_style,
            font_stretch,
        )
        .map(|face| &face.descriptor)
    }

    pub(crate) fn resolve_font_options(
        &self,
        options: &FontOptions<'_>,
    ) -> Result<&LoadedFont, Error> {
        self.resolve_loaded_font(
            options.font_family,
            options.font_name,
            options.font_weight,
            options.font_style,
            options.font_stretch,
        )
        .ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                format!(
                    "no cached font face matched family={:?} name={:?}",
                    options.font_family, options.font_name
                ),
            )
        })
    }

    pub fn text2svg(&self, text: &str, fontsize: f64, fontunit: &str) -> Result<String, Error> {
        let options = self.options().with_font_size(fontsize as f32);
        let layout = self.layout_text_with_fallback(text, options)?;
        glyph_run_to_svg(&layout.run, fontunit)
    }

    /// Builds a color-aware `GlyphRun` using this family's cache.
    ///
    /// The passed options are resolved against this family, and `font_family` defaults to the
    /// family name when omitted.
    pub fn text2glyph_run<'a>(
        &'a self,
        text: &str,
        mut options: FontOptions<'a>,
    ) -> Result<GlyphRun, Error> {
        if options.font_family.is_none() {
            options.font_family = Some(self.name());
        }
        options.font = Some(FontRef::Family(self));
        Ok(self.layout_text_with_fallback(text, options)?.run)
    }

    /// Convenience alias of `FontFamily::text2glyph_run()`.
    pub fn text2commands<'a>(
        &'a self,
        text: &str,
        options: FontOptions<'a>,
    ) -> Result<GlyphRun, Error> {
        self.text2glyph_run(text, options)
    }

    pub fn measure(&self, text: &str) -> Result<f64, Error> {
        Ok(self
            .layout_text_with_fallback(text, self.options())?
            .max_line_width as f64)
    }

    fn layout_text_with_fallback<'a>(
        &'a self,
        text: &str,
        mut options: FontOptions<'a>,
    ) -> Result<FamilyLayoutResult, Error> {
        if self.faces.is_empty() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("no cached font face matched family={:?}", self.name()),
            ));
        }

        if !options.font_size.is_finite() || options.font_size <= 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "font_size must be a positive finite value",
            ));
        }

        options.font = Some(FontRef::Family(self));
        if options.font_family.is_none() {
            options.font_family = Some(self.name());
        }

        let line_height = options.line_height.unwrap_or(options.font_size);
        if !line_height.is_finite() || line_height <= 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "line_height must be a positive finite value",
            ));
        }

        let candidate_indices = self.face_candidate_indices(
            options.font_family,
            options.font_name,
            options.font_weight,
            options.font_style,
            options.font_stretch,
        );
        if candidate_indices.is_empty() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!(
                    "no cached font face matched family={:?} name={:?}",
                    options.font_family, options.font_name
                ),
            ));
        }

        let mut glyphs = Vec::new();
        let mut cursor_x = 0.0f32;
        let mut cursor_y = 0.0f32;
        let mut max_line_width = 0.0f32;
        let mut pending_segment = String::new();
        let mut pending_face = None;

        for unit in fontreader::Font::parse_text_units_for_fallback(text) {
            match unit {
                fontreader::ParsedTextUnit::Newline => {
                    self.flush_family_segment(
                        &mut glyphs,
                        &mut pending_segment,
                        &mut pending_face,
                        &mut cursor_x,
                        cursor_y,
                        options,
                    )?;
                    max_line_width = max_line_width.max(cursor_x);
                    cursor_x = 0.0;
                    cursor_y += line_height;
                }
                fontreader::ParsedTextUnit::Tab => {
                    self.flush_family_segment(
                        &mut glyphs,
                        &mut pending_segment,
                        &mut pending_face,
                        &mut cursor_x,
                        cursor_y,
                        options,
                    )?;
                    cursor_x += line_height * 4.0;
                }
                fontreader::ParsedTextUnit::Glyph { .. } => {
                    let face_index =
                        self.select_face_for_unit(unit, &candidate_indices, options.locale);
                    if pending_face != Some(face_index) {
                        self.flush_family_segment(
                            &mut glyphs,
                            &mut pending_segment,
                            &mut pending_face,
                            &mut cursor_x,
                            cursor_y,
                            options,
                        )?;
                        pending_face = Some(face_index);
                    }
                    push_text_unit(&mut pending_segment, unit);
                }
            }
        }

        self.flush_family_segment(
            &mut glyphs,
            &mut pending_segment,
            &mut pending_face,
            &mut cursor_x,
            cursor_y,
            options,
        )?;
        max_line_width = max_line_width.max(cursor_x);

        Ok(FamilyLayoutResult {
            run: GlyphRun::new(glyphs),
            max_line_width,
        })
    }

    fn flush_family_segment<'a>(
        &'a self,
        glyphs: &mut Vec<PositionedGlyph>,
        pending_segment: &mut String,
        pending_face: &mut Option<usize>,
        cursor_x: &mut f32,
        cursor_y: f32,
        options: FontOptions<'a>,
    ) -> Result<(), Error> {
        let Some(face_index) = *pending_face else {
            pending_segment.clear();
            return Ok(());
        };
        if pending_segment.is_empty() {
            *pending_face = None;
            return Ok(());
        }

        let face = &self.faces[face_index].font;
        let mut segment_options = options;
        segment_options.font = Some(FontRef::Loaded(face));

        let mut segment_run = face.text2glyph_run(pending_segment, segment_options)?;
        let segment_advance = glyph_run_advance_width(&segment_run);
        for glyph in segment_run.glyphs.iter_mut() {
            glyph.x += *cursor_x;
            glyph.y += cursor_y;
        }

        glyphs.extend(segment_run.glyphs);
        *cursor_x += segment_advance;
        pending_segment.clear();
        *pending_face = None;
        Ok(())
    }

    fn find_best_face(
        &self,
        family_name: Option<&str>,
        font_name: Option<&str>,
        font_weight: FontWeight,
        font_style: FontStyle,
        font_stretch: FontStretch,
    ) -> Option<&CachedFontFace> {
        let requested_family = family_name.map(normalize_font_name);
        let requested_name = font_name.map(normalize_font_name);

        self.faces
            .iter()
            .filter_map(|face| {
                let descriptor = &face.descriptor;
                let descriptor_family = normalize_font_name(&descriptor.family_name);
                let owner_family = normalize_font_name(&self.name);
                let family_matches = if requested_name.is_some() {
                    true
                } else if let Some(requested) = requested_family.as_deref() {
                    descriptor_family == requested
                } else {
                    descriptor_family == owner_family
                };

                if let Some(requested_name) = requested_name.as_deref() {
                    let Some(face_name) = descriptor.font_name.as_deref() else {
                        return None;
                    };
                    if normalize_font_name(face_name) != requested_name {
                        return None;
                    }
                } else if !family_matches {
                    return None;
                }

                Some((
                    face_match_score(descriptor, font_weight, font_style, font_stretch),
                    face,
                ))
            })
            .min_by_key(|(score, _)| *score)
            .map(|(_, face)| face)
    }

    fn face_candidate_indices(
        &self,
        family_name: Option<&str>,
        font_name: Option<&str>,
        font_weight: FontWeight,
        font_style: FontStyle,
        font_stretch: FontStretch,
    ) -> Vec<usize> {
        let requested_family = family_name.map(normalize_font_name);
        let requested_name = font_name.map(normalize_font_name);
        let owner_family = normalize_font_name(&self.name);

        let mut candidates: Vec<(u8, u32, usize)> = self
            .faces
            .iter()
            .enumerate()
            .map(|(index, face)| {
                let descriptor = &face.descriptor;
                let descriptor_family = normalize_font_name(&descriptor.family_name);
                let descriptor_name = descriptor.font_name.as_deref().map(normalize_font_name);
                let group = if let Some(requested_name) = requested_name.as_deref() {
                    if descriptor_name.as_deref() == Some(requested_name) {
                        0
                    } else if let Some(requested_family) = requested_family.as_deref() {
                        if descriptor_family == requested_family {
                            1
                        } else if descriptor_family == owner_family {
                            2
                        } else {
                            3
                        }
                    } else if descriptor_family == owner_family {
                        1
                    } else {
                        2
                    }
                } else if let Some(requested_family) = requested_family.as_deref() {
                    if descriptor_family == requested_family {
                        0
                    } else if descriptor_family == owner_family {
                        1
                    } else {
                        2
                    }
                } else if descriptor_family == owner_family {
                    0
                } else {
                    1
                };

                (
                    group,
                    face_match_score(descriptor, font_weight, font_style, font_stretch),
                    index,
                )
            })
            .collect();

        candidates.sort_by_key(|(group, score, index)| (*group, *score, *index));
        candidates.into_iter().map(|(_, _, index)| index).collect()
    }

    fn select_face_for_unit(
        &self,
        unit: fontreader::ParsedTextUnit,
        candidates: &[usize],
        locale: Option<&str>,
    ) -> usize {
        for &index in candidates {
            if self.faces[index]
                .font
                .font()
                .supports_text_unit(unit, false, locale)
            {
                return index;
            }
        }

        candidates[0]
    }
}

fn push_text_unit(target: &mut String, unit: fontreader::ParsedTextUnit) {
    match unit {
        fontreader::ParsedTextUnit::Glyph {
            ch,
            variation_selector,
        } => {
            target.push(ch);
            if variation_selector != '\0' {
                target.push(variation_selector);
            }
        }
        fontreader::ParsedTextUnit::Newline => target.push('\n'),
        fontreader::ParsedTextUnit::Tab => target.push('\t'),
    }
}

fn glyph_run_advance_width(run: &GlyphRun) -> f32 {
    run.glyphs
        .iter()
        .map(|glyph| glyph.x + glyph.glyph.metrics.advance_x)
        .fold(0.0, f32::max)
}

fn normalize_font_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn width_class_to_stretch(width_class: u16) -> f32 {
    match width_class {
        1 => 0.5,
        2 => 0.625,
        3 => 0.75,
        4 => 0.875,
        5 => 1.0,
        6 => 1.125,
        7 => 1.25,
        8 => 1.5,
        9 => 2.0,
        _ => 1.0,
    }
}

fn face_match_score(
    descriptor: &FontFaceDescriptor,
    font_weight: FontWeight,
    font_style: FontStyle,
    font_stretch: FontStretch,
) -> u32 {
    let weight_delta = descriptor.font_weight.0.abs_diff(font_weight.0) as u32;
    let style_penalty = if descriptor.font_style == font_style {
        0
    } else {
        10_000
    };
    let stretch_delta = ((descriptor.font_stretch.0 - font_stretch.0).abs() * 1000.0) as u32;
    style_penalty + weight_delta + stretch_delta
}

fn glyph_run_to_svg(run: &GlyphRun, fontunit: &str) -> Result<String, Error> {
    let Some(bounds) = glyph_run_bounds(run)? else {
        let size = format!("0{}", fontunit);
        return Ok(format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 0 0\"></svg>",
            size, size
        ));
    };

    let view_width = (bounds.max_x - bounds.min_x).max(1.0);
    let view_height = (bounds.max_y - bounds.min_y).max(1.0);
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}{}\" height=\"{}{}\" viewBox=\"{} {} {} {}\">",
        view_width, fontunit, view_height, fontunit, bounds.min_x, bounds.min_y, view_width, view_height
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

/// Reassembles a font file from offset-addressed chunks.
///
/// This is intended for parallel or range-based downloads where the caller gathers pieces of a
/// font file and only calls `load_font_from_buffer()` after the whole payload has been restored.
/// The current WOFF2 decoder still requires a complete byte stream, so this type acts as the
/// assembly layer in front of it.
pub struct ChunkedFontBuffer {
    total_size: usize,
    data: Vec<u8>,
    filled: Vec<bool>,
    filled_len: usize,
}

impl ChunkedFontBuffer {
    pub fn new(total_size: usize) -> Result<Self, Error> {
        if total_size == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "chunked font buffer size must be greater than zero",
            ));
        }

        Ok(Self {
            total_size,
            data: vec![0; total_size],
            filled: vec![false; total_size],
            filled_len: 0,
        })
    }

    pub fn total_size(&self) -> usize {
        self.total_size
    }

    pub fn filled_len(&self) -> usize {
        self.filled_len
    }

    pub fn is_complete(&self) -> bool {
        self.filled_len == self.total_size
    }

    pub fn append(&mut self, offset: usize, bytes: &[u8]) -> Result<(), Error> {
        if bytes.is_empty() {
            return Ok(());
        }

        let end = offset
            .checked_add(bytes.len())
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "chunk offset overflow"))?;
        if end > self.total_size {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "chunk is out of range for the target font buffer",
            ));
        }

        for (index, byte) in bytes.iter().copied().enumerate() {
            let position = offset + index;
            if self.filled[position] {
                if self.data[position] != byte {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "conflicting chunk data for the same byte range",
                    ));
                }
                continue;
            }

            self.data[position] = byte;
            self.filled[position] = true;
            self.filled_len += 1;
        }

        Ok(())
    }

    pub fn missing_ranges(&self) -> Vec<(usize, usize)> {
        let mut ranges = Vec::new();
        let mut start = None;

        for (index, filled) in self.filled.iter().copied().enumerate() {
            match (start, filled) {
                (None, false) => start = Some(index),
                (Some(range_start), true) => {
                    ranges.push((range_start, index));
                    start = None;
                }
                _ => {}
            }
        }

        if let Some(range_start) = start {
            ranges.push((range_start, self.total_size));
        }

        ranges
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, Error> {
        if !self.is_complete() {
            return Err(Error::new(
                ErrorKind::WouldBlock,
                "font buffer is incomplete; append all chunks before decoding",
            ));
        }

        Ok(self.data.clone())
    }

    pub fn load_font(&self) -> Result<LoadedFont, Error> {
        let bytes = self.to_vec()?;
        load_font_from_buffer(&bytes)
    }

    pub fn into_loaded_font(self) -> Result<LoadedFont, Error> {
        if !self.is_complete() {
            return Err(Error::new(
                ErrorKind::WouldBlock,
                "font buffer is incomplete; append all chunks before decoding",
            ));
        }

        load_font_from_buffer(&self.data)
    }
}

pub struct LoadedFont {
    font: fontreader::Font,
}

impl LoadedFont {
    pub fn text2svg(&self, text: &str, fontsize: f64, fontunit: &str) -> Result<String, Error> {
        self.font.text2svg(text, fontsize, fontunit)
    }

    /// Builds a color-aware `GlyphRun`.
    ///
    /// Use this when the caller needs per-layer paint, COLR/CPAL colors, or raster glyph layers.
    pub fn text2glyph_run<'a>(
        &'a self,
        text: &str,
        mut options: FontOptions<'a>,
    ) -> Result<GlyphRun, Error> {
        options.font = Some(FontRef::Loaded(self));
        crate::commands::text2commands(text, options)
    }

    /// Legacy outline-only API.
    ///
    /// This returns `Vec<GlyphCommands>`, keeps `sbix` bitmap payloads in `GlyphCommands::bitmap`,
    /// and does not preserve per-layer paint or full color-layer structure.
    #[deprecated(
        note = "use `LoadedFont::text2glyph_run()` or `fontloader::text2commands(text, FontOptions)` instead"
    )]
    pub fn text2commands(&self, text: &str) -> Result<Vec<GlyphCommands>, Error> {
        self.font.text2commands(text)
    }

    /// Legacy outline-only API.
    ///
    /// This is an alias of `LoadedFont::text2commands()` and does not preserve full color glyph data.
    #[deprecated(
        note = "use `LoadedFont::text2glyph_run()` or `fontloader::text2commands(text, FontOptions)` instead"
    )]
    pub fn text2command(&self, text: &str) -> Result<Vec<GlyphCommands>, Error> {
        self.font.text2command(text)
    }

    pub fn measure(&self, text: &str) -> Result<f64, Error> {
        self.font.measure(text)
    }

    pub fn font(&self) -> &fontreader::Font {
        &self.font
    }
}

#[deprecated(note = "use `load_font_from_file()` instead")]
pub fn fontload_file(path: impl AsRef<Path>) -> Result<LoadedFont, Error> {
    load_font_from_file(path)
}

pub fn load_font_from_file(path: impl AsRef<Path>) -> Result<LoadedFont, Error> {
    let font = fontreader::Font::get_font_from_file(&path.as_ref().to_path_buf())?;
    Ok(LoadedFont { font })
}

#[deprecated(note = "use `load_font_from_buffer()` instead")]
pub fn fontload_buffer(buffer: &[u8]) -> Result<LoadedFont, Error> {
    load_font_from_buffer(buffer)
}

pub fn load_font_from_buffer(buffer: &[u8]) -> Result<LoadedFont, Error> {
    let font = fontreader::Font::get_font_from_buffer(buffer)?;
    Ok(LoadedFont { font })
}

pub fn load_font_from_net(url: &str) -> Result<LoadedFont, Error> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = url;
        return Err(Error::new(
            ErrorKind::Unsupported,
            "network font loading is not supported on wasm32",
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let bytes = fetch_http_font(url)?;
        load_font_from_buffer(&bytes)
    }
}

#[deprecated(note = "use `load_font_from_net()` instead")]
pub fn fontload_net(url: &str) -> Result<LoadedFont, Error> {
    load_font_from_net(url)
}

#[deprecated(note = "use `load_font()` instead")]
pub fn fontload(source: FontSource<'_>) -> Result<LoadedFont, Error> {
    load_font(source)
}

pub fn load_font(source: FontSource<'_>) -> Result<LoadedFont, Error> {
    match source {
        FontSource::File(path) => load_font_from_file(path),
        FontSource::Buffer(buffer) => load_font_from_buffer(buffer),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_http_font(url: &str) -> Result<Vec<u8>, Error> {
    let url = url.strip_prefix("http://").ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            "only http:// URLs are supported for font loading",
        )
    })?;

    let (authority, path) = match url.split_once('/') {
        Some((authority, path)) => (authority, format!("/{}", path)),
        None => (url, "/".to_string()),
    };

    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() && !port.is_empty() => {
            let port = port
                .parse::<u16>()
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "invalid port in http URL"))?;
            (host.to_string(), port)
        }
        _ => (authority.to_string(), 80),
    };

    let mut stream = TcpStream::connect((host.as_str(), port))?;
    let host_header = if port == 80 {
        host.clone()
    } else {
        format!("{}:{}", host, port)
    };
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nAccept: */*\r\n\r\n",
        path, host_header
    );
    stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;

    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "invalid http response"))?
        + 4;

    let header = std::str::from_utf8(&response[..header_end])
        .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid http header"))?;
    if !(header.starts_with("HTTP/1.1 200") || header.starts_with("HTTP/1.0 200")) {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "unexpected http status: {}",
                header.lines().next().unwrap_or("")
            ),
        ));
    }

    Ok(response[header_end..].to_vec())
}
