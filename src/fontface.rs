use crate::commands::{
    FontOptions, FontRef, FontStretch, FontStyle, FontWeight, GlyphRun, PositionedGlyph,
    TextDirection,
};
use crate::fontengine::{glyph_run_to_svg, FontEngine};
use crate::{fontreader, ChunkedFontBuffer};
use std::collections::HashMap;
use std::io::{Error, ErrorKind};

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

#[derive(Debug, Clone)]
pub struct FontFace {
    pub(crate) font: fontreader::Font,
}

impl FontFace {
    pub(crate) fn from_font(font: fontreader::Font) -> Self {
        Self { font }
    }

    pub fn family(&self) -> String {
        self.font.face_family_name()
    }

    pub fn full_name(&self) -> String {
        self.font.face_full_name().unwrap_or_else(|| self.family())
    }

    pub fn weight(&self) -> FontWeight {
        FontWeight(self.font.face_weight_class())
    }

    pub fn stretch(&self) -> FontStretch {
        FontStretch(width_class_to_stretch(self.font.face_width_class()))
    }

    pub fn is_italic(&self) -> bool {
        self.font.face_is_italic()
    }

    pub fn dump(&self) -> String {
        format!(
            "FontFace\nfamily: {}\nfull_name: {}\nweight: {}\nstretch: {:.3}\nitalic: {}\nface_index: {}\nface_count: {}\nformat: {}",
            self.family(),
            self.full_name(),
            self.weight().0,
            self.stretch().0,
            self.is_italic(),
            self.font.get_font_number(),
            self.font.get_font_count(),
            self.font.font_type.to_string()
        )
    }

    pub fn engine(&self) -> FontEngine<'_> {
        FontEngine::new(self)
    }

    pub fn shape(&self, text: &str) -> Result<GlyphRun, Error> {
        self.engine().shape(text)
    }

    pub fn measure(&self, text: &str) -> Result<f64, Error> {
        self.engine().measure(text)
    }

    pub fn render_svg(&self, text: &str) -> Result<String, Error> {
        self.engine().render_svg(text)
    }

    pub fn text2svg(&self, text: &str, fontsize: f64, fontunit: &str) -> Result<String, Error> {
        self.text2svg_with_options(
            text,
            fontunit,
            FontOptions::new(self).with_font_size(fontsize as f32),
        )
    }

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

    pub fn text2glyph_run<'a>(
        &'a self,
        text: &str,
        mut options: FontOptions<'a>,
    ) -> Result<GlyphRun, Error> {
        options.font = Some(FontRef::Loaded(self));
        crate::commands::text2commands(text, options)
    }

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

    pub fn add_face(&mut self, descriptor: FontFaceDescriptor, font: FontFace) -> &FontFace {
        self.faces.push(CachedFontFace { descriptor, font });
        &self.faces.last().expect("face inserted").font
    }

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

    pub fn cached_descriptors(&self) -> Vec<&FontFaceDescriptor> {
        self.faces.iter().map(|face| &face.descriptor).collect()
    }

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

    pub fn text2commands<'a>(
        &'a self,
        text: &str,
        options: FontOptions<'a>,
    ) -> Result<GlyphRun, Error> {
        self.text2glyph_run(text, options)
    }

    pub fn measure(&self, text: &str) -> Result<f64, Error> {
        self.measure_with_options(text, self.options())
    }

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
                        &mut cursor_y,
                        options,
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
                        options,
                    )?;
                    match options.text_direction {
                        TextDirection::LeftToRight => cursor_x += line_height * 4.0,
                        TextDirection::RightToLeft => cursor_x -= line_height * 4.0,
                        TextDirection::TopToBottom => cursor_y += line_height * 4.0,
                    }
                }
                fontreader::ParsedTextUnit::Glyph { .. } => {
                    let face_index =
                        self.select_face_for_unit(&unit, &candidate_indices, options);
                    if pending_face != Some(face_index) {
                        self.flush_family_segment(
                            &mut glyphs,
                            &mut pending_segment,
                            &mut pending_face,
                            &mut cursor_x,
                            &mut cursor_y,
                            options,
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
            options,
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
        options: FontOptions<'_>,
    ) -> usize {
        let is_vertical = options.text_direction.is_vertical();
        for &index in candidates {
            if self.faces[index]
                .font
                .font()
                .supports_text_unit(unit, is_vertical, options.locale)
            {
                return index;
            }
        }

        candidates[0]
    }
}

fn push_text_unit(target: &mut String, unit: &fontreader::ParsedTextUnit) {
    match unit {
        fontreader::ParsedTextUnit::Glyph { text, .. } => target.push_str(text),
        fontreader::ParsedTextUnit::Newline => target.push('\n'),
        fontreader::ParsedTextUnit::Tab => target.push('\t'),
    }
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
