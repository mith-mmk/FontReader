//! Face-level metadata access and family fallback helpers.

use crate::commands::{
    FontOptions, FontRef, FontStretch, FontStyle, FontWeight, GlyphRun, PositionedGlyph,
    TextDirection,
};
use crate::fontengine::{glyph_run_to_svg, FontEngine};
use crate::{fontreader, ChunkedFontBuffer};
use std::collections::HashMap;
use std::io::{Error, ErrorKind};

/// Describes one face inside a [`FontFamily`].
#[derive(Debug, Clone)]
pub struct FontFaceDescriptor {
    /// Family name such as `"Noto Sans JP"`.
    pub family_name: String,
    /// Full face name when available.
    pub font_name: Option<String>,
    /// Requested weight used for selection.
    pub font_weight: FontWeight,
    /// Requested style used for selection.
    pub font_style: FontStyle,
    /// Requested stretch used for selection.
    pub font_stretch: FontStretch,
}

impl FontFaceDescriptor {
    /// Creates a descriptor from a family name.
    pub fn new(family_name: impl Into<String>) -> Self {
        Self {
            family_name: family_name.into(),
            font_name: None,
            font_weight: FontWeight::default(),
            font_style: FontStyle::default(),
            font_stretch: FontStretch::default(),
        }
    }

    /// Sets a preferred face name.
    pub fn with_font_name(mut self, font_name: impl Into<String>) -> Self {
        self.font_name = Some(font_name.into());
        self
    }

    /// Sets the requested weight.
    pub fn with_font_weight(mut self, font_weight: FontWeight) -> Self {
        self.font_weight = font_weight;
        self
    }

    /// Sets the requested style.
    pub fn with_font_style(mut self, font_style: FontStyle) -> Self {
        self.font_style = font_style;
        self
    }

    /// Sets the requested stretch.
    pub fn with_font_stretch(mut self, font_stretch: FontStretch) -> Self {
        self.font_stretch = font_stretch;
        self
    }

    /// Builds a descriptor from an existing [`FontFace`].
    pub fn from_face(face: &FontFace) -> Self {
        Self::from_font(face.font())
    }

    #[cfg(feature = "raw")]
    pub fn from_loaded_font(face: &FontFace) -> Self {
        Self::from_face(face)
    }

    pub(crate) fn from_font(font: &fontreader::Font) -> Self {
        let family_name = font.face_family_name();
        let font_name = font.face_full_name();
        let font_weight = FontWeight(font.face_weight_class());
        let font_style = if font.face_is_italic() {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        };
        let font_stretch = FontStretch(width_class_to_stretch(font.face_width_class()));

        Self {
            family_name,
            font_name,
            font_weight,
            font_style,
            font_stretch,
        }
    }
}

/// One variable-font axis exposed by a face.
#[derive(Debug, Clone)]
pub struct FontVariationAxis {
    /// OpenType tag such as `"wght"` or `"wdth"`.
    pub tag: String,
    /// Human-readable axis name when available.
    pub name: Option<String>,
    pub min_value: f32,
    pub default_value: f32,
    pub max_value: f32,
    pub hidden: bool,
}

/// Public wrapper around one parsed font face.
#[derive(Debug, Clone)]
pub struct FontFace {
    pub(crate) font: fontreader::Font,
}

impl FontFace {
    pub(crate) fn from_font(font: fontreader::Font) -> Self {
        Self { font }
    }

    /// Returns the family name.
    pub fn family(&self) -> String {
        self.font.face_family_name()
    }

    /// Returns the full face name, or the family name if unavailable.
    pub fn full_name(&self) -> String {
        self.font.face_full_name().unwrap_or_else(|| self.family())
    }

    /// Returns the OS/2 weight class as a [`FontWeight`].
    pub fn weight(&self) -> FontWeight {
        FontWeight(self.font.face_weight_class())
    }

    /// Returns the width class mapped to a [`FontStretch`].
    pub fn stretch(&self) -> FontStretch {
        FontStretch(width_class_to_stretch(self.font.face_width_class()))
    }

    /// Returns whether the face is italic.
    pub fn is_italic(&self) -> bool {
        self.font.face_is_italic()
    }

    /// Returns `true` when this face exposes one or more variable-font axes.
    pub fn is_variable(&self) -> bool {
        !self.variation_axes().is_empty()
    }

    /// Returns the available variable-font axes for this face.
    pub fn variation_axes(&self) -> Vec<FontVariationAxis> {
        self.font
            .face_variation_axes()
            .into_iter()
            .map(|axis| FontVariationAxis {
                tag: String::from_utf8_lossy(&axis.tag.to_be_bytes()).into_owned(),
                name: self.font.face_name_by_id(axis.name_id),
                min_value: axis.min_value,
                default_value: axis.default_value,
                max_value: axis.max_value,
                hidden: axis.hidden,
            })
            .collect()
    }

    /// Dumps a small human-readable summary of this face.
    pub fn dump(&self) -> String {
        format!(
            "FontFace\nfamily: {}\nfull_name: {}\nweight: {}\nstretch: {:.3}\nitalic: {}\nvariation_axes: {}\nface_index: {}\nface_count: {}\nformat: {}",
            self.family(),
            self.full_name(),
            self.weight().0,
            self.stretch().0,
            self.is_italic(),
            self.variation_axes().len(),
            self.font.get_font_number(),
            self.font.get_font_count(),
            self.font.font_type.to_string()
        )
    }

    /// Creates a [`FontEngine`] bound to this face.
    pub fn engine(&self) -> FontEngine<'_> {
        FontEngine::new(self)
    }

    /// Shapes text with default engine settings.
    pub fn shape(&self, text: &str) -> Result<GlyphRun, Error> {
        self.engine().shape(text)
    }

    /// Measures text with default engine settings.
    pub fn measure(&self, text: &str) -> Result<f64, Error> {
        self.engine().measure(text)
    }

    /// Renders text to one SVG document with default engine settings.
    pub fn render_svg(&self, text: &str) -> Result<String, Error> {
        self.engine().render_svg(text)
    }

    /// Legacy convenience wrapper for rendering SVG with explicit size and unit.
    /// Renders text to SVG through the family fallback layer.
    pub fn text2svg(&self, text: &str, fontsize: f64, fontunit: &str) -> Result<String, Error> {
        self.text2svg_with_options(
            text,
            fontunit,
            FontOptions::new(self).with_font_size(fontsize as f32),
        )
    }

    /// Renders SVG with fully explicit [`FontOptions`].
    /// Renders text to SVG with explicit family options.
    pub fn text2svg_with_options<'a>(
        &'a self,
        text: &str,
        fontunit: &str,
        mut options: FontOptions<'a>,
    ) -> Result<String, Error> {
        options.font = Some(FontRef::Loaded(self));
        let run = crate::commands::text2commands(text, options)?;
        glyph_run_to_svg(&run, fontunit)
    }

    /// Shapes text with fully explicit [`FontOptions`].
    /// Shapes text through the family fallback layer.
    pub fn text2glyph_run<'a>(
        &'a self,
        text: &str,
        mut options: FontOptions<'a>,
    ) -> Result<GlyphRun, Error> {
        options.font = Some(FontRef::Loaded(self));
        crate::commands::text2commands(text, options)
    }

    /// Measures text with fully explicit [`FontOptions`].
    pub fn measure_with_options<'a>(
        &'a self,
        text: &str,
        mut options: FontOptions<'a>,
    ) -> Result<f64, Error> {
        options.font = Some(FontRef::Loaded(self));
        self.font.measure_with_options(text, &options)
    }

    pub(crate) fn font(&self) -> &fontreader::Font {
        &self.font
    }

    #[cfg(feature = "raw")]
    pub fn raw_font(&self) -> &crate::fontreader::Font {
        &self.font
    }
}

struct CachedFontFace {
    descriptor: FontFaceDescriptor,
    font: FontFace,
}

struct PendingFontFace {
    descriptor: FontFaceDescriptor,
    buffer: ChunkedFontBuffer,
}

struct FamilyLayoutResult {
    run: GlyphRun,
    max_line_width: f32,
}

/// Cache and fallback layer for multiple faces.
pub struct FontFamily {
    name: String,
    faces: Vec<CachedFontFace>,
    pending_faces: HashMap<String, PendingFontFace>,
}

impl FontFamily {
    /// Creates a new named family cache.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            faces: Vec::new(),
            pending_faces: HashMap::new(),
        }
    }

    /// Returns the family cache name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the number of cached faces.
    pub fn cached_faces_len(&self) -> usize {
        self.faces.len()
    }

    /// Returns the number of pending chunked faces.
    pub fn pending_faces_len(&self) -> usize {
        self.pending_faces.len()
    }

    /// Adds a face together with an explicit descriptor.
    pub fn add_face(&mut self, descriptor: FontFaceDescriptor, font: FontFace) -> &FontFace {
        self.faces.push(CachedFontFace { descriptor, font });
        &self.faces.last().expect("face inserted").font
    }

    /// Adds one face and derives its descriptor automatically.
    pub fn add_font_face(&mut self, font: FontFace) -> &FontFace {
        let descriptor = FontFaceDescriptor::from_face(&font);
        self.add_face(descriptor, font)
    }

    #[cfg(feature = "raw")]
    pub fn add_loaded_font(&mut self, font: FontFace) -> &FontFace {
        let face_count = font.font().get_font_count();
        let current_face = font.font().get_font_number();
        let start_index = self.faces.len();

        for face_index in 0..face_count {
            let mut face_font = font.clone();
            if face_font.font.set_font(face_index).is_err() {
                continue;
            }
            let descriptor = FontFaceDescriptor::from_face(&face_font);
            self.faces.push(CachedFontFace {
                descriptor,
                font: face_font,
            });
        }

        &self.faces[start_index + current_face].font
    }

    /// Returns descriptors for all cached faces.
    pub fn cached_descriptors(&self) -> Vec<&FontFaceDescriptor> {
        self.faces.iter().map(|face| &face.descriptor).collect()
    }

    /// Returns default [`FontOptions`] targeting this family.
    pub fn options(&self) -> FontOptions<'_> {
        FontOptions::from_family(self).with_font_family(self.name())
    }

    /// Starts collecting one chunked face.
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

    /// Appends bytes to a pending chunked face.
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

    /// Returns missing byte ranges for a pending chunked face.
    pub fn missing_ranges(&self, face_id: &str) -> Result<Vec<(usize, usize)>, Error> {
        let pending = self.pending_faces.get(face_id).ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                format!("unknown pending font face: {face_id}"),
            )
        })?;
        Ok(pending.buffer.missing_ranges())
    }

    /// Finalizes a chunked face and moves it into the cache.
    pub fn finalize_chunked_face(&mut self, face_id: &str) -> Result<&FontFace, Error> {
        let pending = self.pending_faces.remove(face_id).ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                format!("unknown pending font face: {face_id}"),
            )
        })?;
        let font = pending.buffer.into_font_face()?;
        Ok(self.add_face(pending.descriptor, font))
    }

    /// Resolves the best face for the requested descriptor fields.
    pub fn resolve_face(
        &self,
        family_name: Option<&str>,
        font_name: Option<&str>,
        font_weight: FontWeight,
        font_style: FontStyle,
        font_stretch: FontStretch,
    ) -> Option<&FontFace> {
        self.find_best_face(
            family_name,
            font_name,
            font_weight,
            font_style,
            font_stretch,
        )
        .map(|face| &face.font)
    }

    #[cfg(feature = "raw")]
    pub fn resolve_loaded_font(
        &self,
        family_name: Option<&str>,
        font_name: Option<&str>,
        font_weight: FontWeight,
        font_style: FontStyle,
        font_stretch: FontStretch,
    ) -> Option<&FontFace> {
        self.resolve_face(
            family_name,
            font_name,
            font_weight,
            font_style,
            font_stretch,
        )
    }

    /// Resolves the best matching descriptor.
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
    ) -> Result<&FontFace, Error> {
        self.resolve_face(
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
        self.text2svg_with_options(text, fontunit, options)
    }

    pub fn text2svg_with_options<'a>(
        &'a self,
        text: &str,
        fontunit: &str,
        mut options: FontOptions<'a>,
    ) -> Result<String, Error> {
        if options.font_family.is_none() {
            options.font_family = Some(self.name());
        }
        let layout = self.layout_text_with_fallback(text, options)?;
        glyph_run_to_svg(&layout.run, fontunit)
    }

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

    /// Alias for [`FontFamily::text2glyph_run`].
    pub fn text2commands<'a>(
        &'a self,
        text: &str,
        options: FontOptions<'a>,
    ) -> Result<GlyphRun, Error> {
        self.text2glyph_run(text, options)
    }

    /// Measures text with default family options.
    pub fn measure(&self, text: &str) -> Result<f64, Error> {
        self.measure_with_options(text, self.options())
    }

    /// Measures text through the family fallback layer.
    pub fn measure_with_options<'a>(
        &'a self,
        text: &str,
        mut options: FontOptions<'a>,
    ) -> Result<f64, Error> {
        if options.font_family.is_none() {
            options.font_family = Some(self.name());
        }
        Ok(self
            .layout_text_with_fallback(text, options)?
            .max_line_width as f64)
    }

    #[cfg(all(test, feature = "raw"))]
    pub(crate) fn debug_face_indices_for_text(
        &self,
        text: &str,
        options: FontOptions<'_>,
    ) -> Result<Vec<usize>, Error> {
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

        let mut pending_face: Option<usize> = None;
        let mut face_indices = Vec::new();
        for unit in fontreader::Font::parse_text_units_for_fallback(text) {
            match unit {
                fontreader::ParsedTextUnit::Newline | fontreader::ParsedTextUnit::Tab => {
                    pending_face = None;
                }
                fontreader::ParsedTextUnit::Glyph { .. } => {
                    let face_index = if let Some(current_face) = pending_face {
                        if unit_prefers_face_continuity(&unit, &options)
                            && self.faces[current_face].font.font().supports_text_unit(
                                &unit,
                                options.text_direction,
                                options.locale,
                                options.font_variant,
                            )
                        {
                            current_face
                        } else {
                            self.select_face_for_unit(&unit, &candidate_indices, &options)
                        }
                    } else {
                        self.select_face_for_unit(&unit, &candidate_indices, &options)
                    };
                    pending_face = Some(face_index);
                    face_indices.push(face_index);
                }
            }
        }

        Ok(face_indices)
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
        let mut pending_face: Option<usize> = None;

        for unit in fontreader::Font::parse_text_units_for_fallback(text) {
            match unit {
                fontreader::ParsedTextUnit::Newline => {
                    self.flush_family_segment(
                        &mut glyphs,
                        &mut pending_segment,
                        &mut pending_face,
                        &mut cursor_x,
                        &mut cursor_y,
                        &options,
                    )?;
                    max_line_width = max_line_width.max(cursor_inline_extent(
                        cursor_x,
                        cursor_y,
                        options.text_direction,
                    ));
                    match options.text_direction {
                        TextDirection::LeftToRight | TextDirection::RightToLeft => {
                            cursor_x = 0.0;
                            cursor_y += line_height;
                        }
                        TextDirection::TopToBottom => {
                            cursor_x -= line_height;
                            cursor_y = 0.0;
                        }
                    }
                }
                fontreader::ParsedTextUnit::Tab => {
                    self.flush_family_segment(
                        &mut glyphs,
                        &mut pending_segment,
                        &mut pending_face,
                        &mut cursor_x,
                        &mut cursor_y,
                        &options,
                    )?;
                    match options.text_direction {
                        TextDirection::LeftToRight => cursor_x += line_height * 4.0,
                        TextDirection::RightToLeft => cursor_x -= line_height * 4.0,
                        TextDirection::TopToBottom => cursor_y += line_height * 4.0,
                    }
                }
                fontreader::ParsedTextUnit::Glyph { .. } => {
                    let face_index = if let Some(current_face) = pending_face {
                        if unit_prefers_face_continuity(&unit, &options)
                            && self.faces[current_face].font.font().supports_text_unit(
                                &unit,
                                options.text_direction,
                                options.locale,
                                options.font_variant,
                            )
                        {
                            current_face
                        } else {
                            self.select_face_for_unit(&unit, &candidate_indices, &options)
                        }
                    } else {
                        self.select_face_for_unit(&unit, &candidate_indices, &options)
                    };
                    if pending_face != Some(face_index) {
                        self.flush_family_segment(
                            &mut glyphs,
                            &mut pending_segment,
                            &mut pending_face,
                            &mut cursor_x,
                            &mut cursor_y,
                            &options,
                        )?;
                        pending_face = Some(face_index);
                    }
                    push_text_unit(&mut pending_segment, &unit);
                }
            }
        }

        self.flush_family_segment(
            &mut glyphs,
            &mut pending_segment,
            &mut pending_face,
            &mut cursor_x,
            &mut cursor_y,
            &options,
        )?;
        max_line_width = max_line_width.max(cursor_inline_extent(
            cursor_x,
            cursor_y,
            options.text_direction,
        ));

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
        cursor_y: &mut f32,
        options: &FontOptions<'a>,
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
        let mut segment_options = options.clone();
        segment_options.font = Some(FontRef::Loaded(face));

        let mut segment_run = face.text2glyph_run(pending_segment, segment_options)?;
        let (segment_advance_x, segment_advance_y) =
            glyph_run_cursor_delta(&segment_run, options.text_direction);
        for glyph in segment_run.glyphs.iter_mut() {
            glyph.x += *cursor_x;
            glyph.y += *cursor_y;
        }

        glyphs.extend(segment_run.glyphs);
        *cursor_x += segment_advance_x;
        *cursor_y += segment_advance_y;
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
        unit: &fontreader::ParsedTextUnit,
        candidates: &[usize],
        options: &FontOptions<'_>,
    ) -> usize {
        let prefer_color = text_unit_prefers_color_glyph(unit);
        let mut best_face = None;
        let mut best_rank = 0u8;

        for &index in candidates {
            let support = self.faces[index].font.font().text_unit_support(
                unit,
                options.text_direction,
                options.locale,
                options.font_variant,
            );
            if !support.is_supported() {
                continue;
            }

            let rank = face_support_rank(support, prefer_color);
            if best_face.is_none() || rank > best_rank {
                best_face = Some(index);
                best_rank = rank;
            }
        }

        best_face.unwrap_or(candidates[0])
    }
}

fn push_text_unit(target: &mut String, unit: &fontreader::ParsedTextUnit) {
    match unit {
        fontreader::ParsedTextUnit::Glyph { text, .. } => target.push_str(text),
        fontreader::ParsedTextUnit::Newline => target.push('\n'),
        fontreader::ParsedTextUnit::Tab => target.push('\t'),
    }
}

fn unit_prefers_face_continuity(
    unit: &fontreader::ParsedTextUnit,
    options: &FontOptions<'_>,
) -> bool {
    let fontreader::ParsedTextUnit::Glyph { text, .. } = unit else {
        return false;
    };

    text_contains_combining_mark(text)
        || (options.text_direction.is_right_to_left() && text_contains_contextual_rtl_script(text))
}

fn text_unit_prefers_color_glyph(unit: &fontreader::ParsedTextUnit) -> bool {
    let fontreader::ParsedTextUnit::Glyph { text, .. } = unit else {
        return false;
    };

    text.chars().any(|ch| {
        matches!(ch as u32, 0xFE00..=0xFE0F | 0xE0100..=0xE01EF)
            || matches!(ch as u32, 0x1F3FB..=0x1F3FF)
            || ch == '\u{200D}'
            || ch == '\u{20E3}'
            || matches!(ch as u32, 0x1F1E6..=0x1F1FF)
            || matches!(ch as u32, 0xE0020..=0xE007E)
            || ch == '\u{E007F}'
            || matches!(ch as u32, 0x1F000..=0x1FAFF | 0x2600..=0x27BF)
    })
}

fn face_support_rank(support: fontreader::TextUnitSupport, prefer_color: bool) -> u8 {
    match (prefer_color, support.has_outline, support.has_color) {
        (true, true, true) => 4,
        (true, false, true) => 3,
        (true, true, false) => 2,
        (false, true, false) => 4,
        (false, true, true) => 3,
        (false, false, true) => 2,
        _ => 0,
    }
}

fn text_contains_combining_mark(text: &str) -> bool {
    text.chars().any(is_combining_mark)
}

fn is_combining_mark(ch: char) -> bool {
    matches!(
        ch as u32,
        0x0300..=0x036F
            | 0x0483..=0x0489
            | 0x0591..=0x05BD
            | 0x05BF
            | 0x05C1..=0x05C2
            | 0x05C4..=0x05C5
            | 0x05C7
            | 0x0610..=0x061A
            | 0x064B..=0x065F
            | 0x0670
            | 0x06D6..=0x06DC
            | 0x06DF..=0x06E4
            | 0x06E7..=0x06E8
            | 0x06EA..=0x06ED
            | 0x0711
            | 0x0730..=0x074A
            | 0x07A6..=0x07B0
            | 0x07EB..=0x07F3
            | 0x0816..=0x0819
            | 0x081B..=0x0823
            | 0x0825..=0x0827
            | 0x0829..=0x082D
            | 0x0859..=0x085B
            | 0x08D3..=0x08E1
            | 0x08E3..=0x08FF
            | 0x0F18..=0x0F19
            | 0x0F35
            | 0x0F37
            | 0x0F39
            | 0x0F71..=0x0F7E
            | 0x0F80..=0x0F84
            | 0x0F86..=0x0F87
            | 0x0F8D..=0x0FBC
            | 0x1AB0..=0x1AFF
            | 0x1DC0..=0x1DFF
            | 0x20D0..=0x20FF
            | 0x3099..=0x309A
            | 0xFE20..=0xFE2F
    )
}

fn text_contains_contextual_rtl_script(text: &str) -> bool {
    text.chars().any(is_contextual_rtl_script)
}

fn is_contextual_rtl_script(ch: char) -> bool {
    matches!(
        ch as u32,
        0x0590..=0x05FF
            | 0x0600..=0x06FF
            | 0x0700..=0x074F
            | 0x0750..=0x077F
            | 0x0780..=0x07BF
            | 0x07C0..=0x07FF
            | 0x0800..=0x083F
            | 0x0840..=0x085F
            | 0x0860..=0x086F
            | 0x0870..=0x089F
            | 0x08A0..=0x08FF
            | 0xFB1D..=0xFDFF
            | 0xFE70..=0xFEFF
            | 0x10E60..=0x10E7F
            | 0x1EE00..=0x1EEFF
    )
}

fn glyph_run_cursor_delta(run: &GlyphRun, text_direction: TextDirection) -> (f32, f32) {
    match text_direction {
        TextDirection::LeftToRight => (
            run.glyphs
                .iter()
                .map(|glyph| glyph.glyph.metrics.advance_x)
                .sum(),
            run.glyphs
                .iter()
                .map(|glyph| glyph.glyph.metrics.advance_y)
                .sum(),
        ),
        TextDirection::RightToLeft => (
            -run.glyphs
                .iter()
                .map(|glyph| glyph.glyph.metrics.advance_x)
                .sum::<f32>(),
            run.glyphs
                .iter()
                .map(|glyph| glyph.glyph.metrics.advance_y)
                .sum(),
        ),
        TextDirection::TopToBottom => (
            run.glyphs
                .iter()
                .map(|glyph| glyph.glyph.metrics.advance_x)
                .sum(),
            run.glyphs
                .iter()
                .map(|glyph| glyph.glyph.metrics.advance_y)
                .sum(),
        ),
    }
}

fn cursor_inline_extent(cursor_x: f32, cursor_y: f32, text_direction: TextDirection) -> f32 {
    match text_direction {
        TextDirection::LeftToRight => cursor_x.max(0.0),
        TextDirection::RightToLeft => (-cursor_x).max(0.0),
        TextDirection::TopToBottom => cursor_y.max(0.0),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_prefers_face_continuity_for_combining_marks() {
        let unit = crate::fontreader::ParsedTextUnit::Glyph {
            text: "e\u{0301}".to_string(),
            ch: 'e',
            variation_selector: '\0',
        };

        let family = FontFamily::new("Test");
        let options = FontOptions::from_font_ref(FontRef::Family(&family));
        assert!(unit_prefers_face_continuity(&unit, &options));
    }

    #[test]
    fn unit_prefers_face_continuity_for_rtl_contextual_scripts() {
        let unit = crate::fontreader::ParsedTextUnit::Glyph {
            text: "ب".to_string(),
            ch: 'ب',
            variation_selector: '\0',
        };

        let family = FontFamily::new("Test");
        let options = FontOptions::from_font_ref(FontRef::Family(&family)).with_right_to_left();
        assert!(unit_prefers_face_continuity(&unit, &options));
    }

    #[test]
    fn unit_does_not_prefer_face_continuity_for_plain_latin() {
        let unit = crate::fontreader::ParsedTextUnit::Glyph {
            text: "A".to_string(),
            ch: 'A',
            variation_selector: '\0',
        };

        let family = FontFamily::new("Test");
        let options = FontOptions::from_font_ref(FontRef::Family(&family));
        assert!(!unit_prefers_face_continuity(&unit, &options));
    }
}
