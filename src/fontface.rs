//! Face-level metadata access and family fallback helpers.

use crate::commands::{
    FontOptions, FontRef, FontStretch, FontStyle, FontWeight, GlyphRun, PositionedGlyph,
    TextDirection,
};
use crate::fontengine::{glyph_run_to_svg, FontEngine};
use crate::{fontreader, ChunkedFontBuffer, FontFile};
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
            self.font.font_type
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
        options: FontOptions<'a>,
    ) -> Result<f64, Error> {
        self.engine().with_options(options).measure(text)
    }

    /// Measures logical and ink extents from one shaped run.
    pub fn measure_layout<'a>(
        &'a self,
        text: &str,
        options: FontOptions<'a>,
    ) -> Result<crate::LayoutMetrics, Error> {
        self.engine().with_options(options).measure_layout(text)
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
    sources: Vec<crate::mapped::GlyphSource>,
    ligature_carets: Vec<Option<Vec<f32>>>,
    max_line_width: f32,
    #[cfg(feature = "svg-fonts")]
    svg_placements: crate::commands::SvgPlacements,
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
        let index = self.faces.len();
        self.faces.push(CachedFontFace { descriptor, font });
        &self.faces[index].font
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

        let mut selected_index = None;
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
            if face_index == current_face {
                selected_index = self.faces.len().checked_sub(1);
            }
        }

        if selected_index.is_none() {
            let descriptor = FontFaceDescriptor::from_face(&font);
            self.faces.push(CachedFontFace { descriptor, font });
            selected_index = self.faces.len().checked_sub(1);
        }

        &self.faces[selected_index.unwrap_or(start_index)].font
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
        if self.pending_faces.contains_key(&face_id) {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                format!("pending font face already exists: {face_id}"),
            ));
        }
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
        let pending = self.pending_faces.get(face_id).ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                format!("unknown pending font face: {face_id}"),
            )
        })?;
        let descriptor = pending.descriptor.clone();
        let bytes = pending.buffer.to_vec()?;
        let font = FontFile::from_buffer(&bytes)?.current_face()?;
        self.pending_faces.remove(face_id);
        Ok(self.add_face(descriptor, font))
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

    /// Shapes with the same fallback selection and layout as
    /// [`Self::text2glyph_run`], retaining the resolved SVG transform for each
    /// layer from its selected face, including the final segment origin.
    #[cfg(feature = "svg-fonts")]
    pub fn shape_with_svg_context<'a>(
        &'a self,
        text: &str,
        options: FontOptions<'a>,
    ) -> Result<crate::SvgGlyphRun, Error> {
        let result = self.layout_text_with_fallback(text, options)?;
        Ok(crate::SvgGlyphRun {
            run: result.run,
            placements: result.svg_placements,
        })
    }

    /// Shapes with original UTF-8 source ranges and fallback-face provenance.
    ///
    /// Face indices identify this family's insertion order; run indices identify
    /// consecutive shaped segments. Neither is a globally stable font identity.
    pub fn shape_mapped<'a>(
        &'a self,
        text: &str,
        mut options: FontOptions<'a>,
    ) -> Result<crate::mapped::ShapedText, Error> {
        if options.font_family.is_none() {
            options.font_family = Some(self.name());
        }
        let direction = options.text_direction;
        let result = self.layout_text_with_fallback_impl(text, options, true)?;
        let shaped = crate::mapped::ShapedText::new(text, result.run, result.sources, direction)
            .with_ligature_carets(result.ligature_carets);
        #[cfg(feature = "svg-fonts")]
        let shaped = shaped.with_svg_placements(result.svg_placements);
        Ok(shaped)
    }

    /// Shapes single-line mixed vertical LTR text using [`FontFamily`] fallback.
    ///
    /// Each Unicode `Vertical_Orientation` run is segmented by face support,
    /// preserving original UTF-8 ranges and `face_index` provenance.
    pub fn shape_vertical_mixed_ltr_mapped<'a>(
        &'a self,
        text: &str,
        mut options: FontOptions<'a>,
    ) -> Result<crate::vertical_mapped::VerticalMappedLayout, Error> {
        use crate::vertical_mapped::{VerticalGroup, VerticalMappedLayout};
        crate::fontengine::validate_single_line(text, "mixed vertical LTR")?;
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
        if options.font_family.is_none() {
            options.font_family = Some(self.name());
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
        let mut sources = Vec::new();
        let mut mapped_groups = Vec::new();
        let (mut pen_x, mut pen_y) = (0.0f32, 0.0f32);
        let mut byte_start = 0usize;
        let mut orient_groups = crate::fontengine::mixed_orientation_groups(text);
        if orient_groups.is_empty() {
            orient_groups.push((text, false));
        }

        let mut group_index = 0usize;
        for (orient_text, sideways) in orient_groups {
            let sub_segments =
                self.partition_orientation_group_by_face(orient_text, &candidate_indices, &options);
            for (sub_text, face_index) in sub_segments {
                let face = &self.faces[face_index].font;
                let mut sub_options = options.clone();
                sub_options.font = Some(FontRef::Loaded(face));
                let output = if sideways {
                    face.font()
                        .text2glyph_run_sideways_ltr_with_carets(sub_text, &sub_options)?
                } else {
                    face.font()
                        .text2glyph_run_mixed_upright_with_carets(sub_text, &sub_options)?
                };
                let local_run = output.run;
                let ranges = output.ranges;
                let shift_x = if sideways {
                    local_run
                        .glyphs
                        .first()
                        .and_then(|glyph| glyph.glyph.font)
                        .map(|metrics| -(metrics.ascent - metrics.descent) / 2.0)
                        .unwrap_or(0.0)
                } else {
                    0.0
                };
                if !shift_x.is_finite() {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "mixed vertical centering overflow",
                    ));
                }
                let local_sources = ranges
                    .iter()
                    .cloned()
                    .map(|range| crate::GlyphSource::new(range, face_index, group_index))
                    .collect();
                let local = crate::ShapedText::new(
                    sub_text,
                    local_run.clone(),
                    local_sources,
                    if sideways {
                        TextDirection::LeftToRight
                    } else {
                        TextDirection::TopToBottom
                    },
                )
                .with_ligature_carets(output.ligature_carets);

                let mut run = if sideways {
                    crate::fontengine::rotate_sideways_run(local_run, false)?
                } else {
                    let mut run = local_run;
                    crate::fontengine::validate_vertical_run(
                        &mut run,
                        "mixed vertical LTR",
                        false,
                    )?;
                    run
                };
                let translation = (pen_x + shift_x, pen_y);
                if !translation.0.is_finite() || !translation.1.is_finite() {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "mixed vertical translation overflow",
                    ));
                }
                let (mut advance_x, mut advance_y) = (0.0f32, 0.0f32);
                for glyph in &mut run.glyphs {
                    glyph.x += translation.0;
                    glyph.y += translation.1;
                    advance_x += glyph.glyph.metrics.advance_x;
                    advance_y += glyph.glyph.metrics.advance_y;
                    if !glyph.x.is_finite()
                        || !glyph.y.is_finite()
                        || !glyph.glyph.metrics.advance_x.is_finite()
                        || !glyph.glyph.metrics.advance_y.is_finite()
                        || !advance_x.is_finite()
                        || !advance_y.is_finite()
                    {
                        return Err(Error::new(
                            ErrorKind::InvalidInput,
                            "mixed vertical glyph or advance overflow",
                        ));
                    }
                }
                mapped_groups.push(VerticalGroup::new(
                    byte_start..byte_start + sub_text.len(),
                    local,
                    sideways,
                    translation,
                ));
                sources.extend(ranges.into_iter().map(|range| {
                    crate::GlyphSource::new(
                        byte_start + range.start..byte_start + range.end,
                        face_index,
                        group_index,
                    )
                }));
                glyphs.extend(run.glyphs);
                pen_x += advance_x;
                pen_y += advance_y;
                if !pen_x.is_finite() || !pen_y.is_finite() {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "mixed vertical pen overflow",
                    ));
                }
                byte_start += sub_text.len();
                group_index += 1;
            }
        }

        Ok(VerticalMappedLayout::new(
            text,
            GlyphRun::new(glyphs),
            sources,
            mapped_groups,
        ))
    }

    pub(crate) fn shape_tate_chu_yoko_ltr_with_face<'a>(
        &'a self,
        text: &str,
        mut options: FontOptions<'a>,
    ) -> Result<(GlyphRun, usize), Error> {
        if self.faces.is_empty() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("no cached font face matched family={:?}", self.name()),
            ));
        }
        if options.font_family.is_none() {
            options.font_family = Some(self.name());
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
        let mut chosen = candidate_indices[0];
        if let Some(first_unit) = fontreader::Font::parse_text_units_for_fallback(text)
            .into_iter()
            .next()
        {
            chosen = self.select_face_for_unit(&first_unit, &candidate_indices, &options);
        }
        let run = self.faces[chosen]
            .font
            .engine()
            .with_options(options)
            .shape_tate_chu_yoko_ltr(text)?;
        Ok((run, chosen))
    }

    fn partition_orientation_group_by_face<'a>(
        &self,
        group_text: &'a str,
        candidate_indices: &[usize],
        options: &FontOptions<'_>,
    ) -> Vec<(&'a str, usize)> {
        if group_text.is_empty() {
            return vec![("", candidate_indices[0])];
        }
        let mut segments = Vec::new();
        let mut seg_start = 0usize;
        let mut offset = 0usize;
        let mut current_face: Option<usize> = None;
        for grapheme in crate::unicode::graphemes::extended_graphemes(group_text) {
            let unit = fontreader::Font::parse_text_units_for_fallback(grapheme)
                .into_iter()
                .next();
            let face_index = match unit {
                Some(ref u) => {
                    self.select_face_with_continuity(u, candidate_indices, options, current_face)
                }
                None => current_face.unwrap_or(candidate_indices[0]),
            };
            if let Some(prev) = current_face {
                if prev != face_index {
                    segments.push((&group_text[seg_start..offset], prev));
                    seg_start = offset;
                }
            }
            current_face = Some(face_index);
            offset += grapheme.len();
        }
        if let Some(prev) = current_face {
            segments.push((&group_text[seg_start..offset], prev));
        }
        segments
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
                    let face_index = self.select_face_with_continuity(
                        &unit,
                        &candidate_indices,
                        &options,
                        pending_face,
                    );
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
        options: FontOptions<'a>,
    ) -> Result<FamilyLayoutResult, Error> {
        self.layout_text_with_fallback_impl(text, options, false)
    }

    fn layout_text_with_fallback_impl<'a>(
        &'a self,
        text: &str,
        mut options: FontOptions<'a>,
        mapped: bool,
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
        let mut provenance = SegmentProvenance::new(mapped);
        let mut cursor_x = 0.0f32;
        let mut cursor_y = 0.0f32;
        let mut max_line_width = 0.0f32;
        let mut pending_segment = String::new();
        let mut pending_face: Option<usize> = None;

        for (unit, source) in sourced_text_units(text) {
            match unit {
                fontreader::ParsedTextUnit::Newline => {
                    self.flush_family_segment(
                        &mut glyphs,
                        &mut provenance,
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
                        &mut provenance,
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
                    let face_index = self.select_face_with_continuity(
                        &unit,
                        &candidate_indices,
                        &options,
                        pending_face,
                    );
                    if pending_face != Some(face_index) {
                        self.flush_family_segment(
                            &mut glyphs,
                            &mut provenance,
                            &mut pending_segment,
                            &mut pending_face,
                            &mut cursor_x,
                            &mut cursor_y,
                            &options,
                        )?;
                        pending_face = Some(face_index);
                    }
                    provenance.append(source);
                    push_text_unit(&mut pending_segment, &unit);
                }
            }
        }

        self.flush_family_segment(
            &mut glyphs,
            &mut provenance,
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
            sources: provenance.sources,
            ligature_carets: provenance.ligature_carets,
            #[cfg(feature = "svg-fonts")]
            svg_placements: provenance.svg_placements,
            max_line_width,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn flush_family_segment<'a>(
        &'a self,
        glyphs: &mut Vec<PositionedGlyph>,
        provenance: &mut SegmentProvenance,
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

        let mut segment_run = if provenance.enabled {
            let mapped = face
                .font()
                .text2glyph_run_with_carets(pending_segment, &segment_options)?;
            provenance.ligature_carets.extend(mapped.ligature_carets);
            provenance.finish(mapped.ranges, face_index);
            mapped.run
        } else {
            face.font()
                .text2glyph_run(pending_segment, &segment_options)?
        };
        let (segment_advance_x, segment_advance_y) =
            glyph_run_cursor_delta(&segment_run, options.text_direction);
        for glyph in segment_run.glyphs.iter_mut() {
            glyph.x += *cursor_x;
            glyph.y += *cursor_y;
        }

        #[cfg(feature = "svg-fonts")]
        provenance.svg_placements.append_run(
            face.font(),
            &segment_options,
            &segment_run.glyphs,
            glyphs.len(),
            false,
        )?;
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
                    let face_name = descriptor.font_name.as_deref()?;
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

    fn select_face_with_continuity(
        &self,
        unit: &fontreader::ParsedTextUnit,
        candidates: &[usize],
        options: &FontOptions<'_>,
        previous: Option<usize>,
    ) -> usize {
        if let Some(index) = previous {
            let current = self.faces[index].font.font();
            if unit_prefers_face_continuity(unit, options)
                && current.supports_text_unit(
                    unit,
                    options.text_direction,
                    options.locale,
                    options.font_variant,
                )
            {
                // Keep the old fast path for ordinary marked/RTL clusters and
                // exact sequences. Scan only when base fallback could hide a
                // genuinely supported variation in another face.
                if current.has_exact_variations(unit) {
                    return index;
                }
                let selected = self.select_face_for_unit(unit, candidates, options);
                return if self.faces[selected].font.font().has_exact_variations(unit) {
                    selected
                } else {
                    index
                };
            }
        }
        self.select_face_for_unit(unit, candidates, options)
    }

    fn select_face_for_unit(
        &self,
        unit: &fontreader::ParsedTextUnit,
        candidates: &[usize],
        options: &FontOptions<'_>,
    ) -> usize {
        let prefer_color = text_unit_prefers_color_glyph(unit);
        let mut best_face = None;
        let mut best_rank = (false, 0u8);

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

            // Prefer a genuine supported sequence over nominal-base fallback,
            // but only after the complete cluster passes shaped coverage.
            let rank = (
                self.faces[index].font.font().has_exact_variations(unit),
                face_support_rank(support, prefer_color),
            );
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
    name.trim().to_lowercase()
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

// The legacy parser may remove CR or leading orphan variation selectors.
// Each retained byte keeps its ORIGINAL grapheme envelope, including a removed
// leading selector when that same grapheme still contains a rendered mark.
// Entirely discarded graphemes contribute no entry and remain source-only gaps.
pub(crate) struct SourceCluster {
    range: std::ops::Range<usize>,
    retained_len: usize,
}

pub(crate) fn sourced_text_units(text: &str) -> Vec<(fontreader::ParsedTextUnit, SourceCluster)> {
    let mut offset = 0;
    let mut result = Vec::new();
    for grapheme in crate::unicode::graphemes::extended_graphemes(text) {
        for unit in fontreader::Font::parse_text_units_for_fallback(grapheme) {
            let retained_len = match &unit {
                fontreader::ParsedTextUnit::Glyph { text, .. } => text.len(),
                _ => grapheme.len(),
            };
            let source = SourceCluster {
                range: offset..offset + grapheme.len(),
                retained_len,
            };
            result.push((unit, source));
        }
        offset += grapheme.len();
    }
    result
}

pub(crate) struct SegmentProvenance {
    enabled: bool,
    bytes: Vec<std::ops::Range<usize>>,
    pub(crate) sources: Vec<crate::mapped::GlyphSource>,
    pub(crate) ligature_carets: Vec<Option<Vec<f32>>>,
    #[cfg(feature = "svg-fonts")]
    pub(crate) svg_placements: crate::commands::SvgPlacements,
    run_index: usize,
}

impl SegmentProvenance {
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            bytes: Vec::new(),
            sources: Vec::new(),
            ligature_carets: Vec::new(),
            #[cfg(feature = "svg-fonts")]
            svg_placements: Default::default(),
            run_index: 0,
        }
    }

    pub(crate) fn append(&mut self, source: SourceCluster) {
        if self.enabled {
            self.bytes
                .extend(std::iter::repeat_n(source.range, source.retained_len));
        }
    }

    pub(crate) fn finish(&mut self, ranges: Vec<std::ops::Range<usize>>, face_index: usize) {
        for range in ranges {
            // Core ranges are nonempty, in-bounds UTF-8 ranges over the pending
            // segment. Preserve gaps from removed controls as conservative spans.
            let source = self.bytes[range.start].start..self.bytes[range.end - 1].end;
            self.sources.push(crate::mapped::GlyphSource::new(
                source,
                face_index,
                self.run_index,
            ));
        }
        self.bytes.clear();
        self.run_index += 1;
    }
}

#[cfg(test)]
mod mapped_fallback_tests {
    use super::*;
    const FONT: &[u8] = include_bytes!("../tests/fixtures/european-fallback.ttc");

    fn family() -> FontFamily {
        let file = crate::FontFile::from_buffer(FONT).unwrap();
        let mut family = FontFamily::new("Primary");
        family.add_face(FontFaceDescriptor::new("Primary"), file.face(0).unwrap());
        family.add_face(FontFaceDescriptor::new("Fallback"), file.face(1).unwrap());
        family
    }

    #[test]
    fn mapped_adapters_preserve_original_grapheme_with_trimmed_selector() {
        let file = crate::FontFile::from_buffer(FONT).unwrap();
        let face = file.face(1).unwrap();
        let mut family = FontFamily::new("Single");
        family.add_face(FontFaceDescriptor::new("Single"), face.clone());
        let composite = crate::CompositeFont::new(vec![&face]).unwrap();
        for (text, expected) in [
            ("\u{fe0f}\u{0308}B", vec![0..5, 5..6]),
            ("\u{fe0f}\u{e0100}\u{0308}B", vec![0..9, 9..10]),
            ("\u{fe0f}B", std::iter::once(3..4).collect::<Vec<_>>()),
            ("\u{fe0f}", vec![]),
        ] {
            let engine = face.engine().shape_mapped(text).unwrap();
            let family = family.shape_mapped(text, family.options()).unwrap();
            let composite = composite.shape_mapped_ltr(text).unwrap();
            for mapped in [&engine, &family, &composite] {
                assert_eq!(
                    mapped
                        .glyph_sources()
                        .iter()
                        .map(|s| s.byte_range())
                        .collect::<Vec<_>>(),
                    expected,
                    "{text:?}"
                );
                assert!(mapped.glyph_sources().iter().all(|s| {
                    let r = s.byte_range();
                    text.is_char_boundary(r.start) && text.is_char_boundary(r.end)
                }));
            }
        }
    }

    #[test]
    fn mapped_family_preserves_original_bytes_and_legacy_geometry() {
        let family = family();
        for text in [
            "Ba\u{308}Bßẞ",
            "B\r\nä\tB",
            "B\räB",
            "\u{fe0f}äB",
            "",
            "\r\u{fe0f}",
        ] {
            let options = family.options().with_font_size(1000.0);
            let mapped = family.shape_mapped(text, options.clone()).unwrap();
            let legacy = family.text2glyph_run(text, options).unwrap();
            assert_eq!(format!("{:?}", mapped.glyph_run()), format!("{legacy:?}"));
            assert_eq!(mapped.text(), text);
            assert_eq!(mapped.glyph_sources().len(), legacy.glyphs.len());
            for source in mapped.glyph_sources() {
                let slice = &text[source.byte_range()];
                assert!(!slice.is_empty());
                assert!(!slice.contains('\n') && !slice.contains('\t'));
            }
        }
        let mapped = family.shape_mapped("Ba\u{308}B", family.options()).unwrap();
        assert_eq!(mapped.glyph_sources().first().unwrap().byte_range(), 0..1);
        assert_eq!(mapped.glyph_sources().last().unwrap().byte_range(), 4..5);
        for source in &mapped.glyph_sources()[1..mapped.glyph_sources().len() - 1] {
            assert_eq!(source.byte_range(), 1..4);
            assert_eq!(source.face_index(), 1);
        }
    }

    #[test]
    fn mapped_family_retains_offsets_across_controls_in_all_directions() {
        let family = family();
        let text = "B\r\nä\tB";
        for direction in [
            TextDirection::LeftToRight,
            TextDirection::RightToLeft,
            TextDirection::TopToBottom,
        ] {
            let options = family.options().with_text_direction(direction);
            let mapped = family.shape_mapped(text, options.clone()).unwrap();
            assert_eq!(
                format!("{:?}", mapped.glyph_run()),
                format!("{:?}", family.text2glyph_run(text, options).unwrap())
            );
            let ranges: Vec<_> = mapped
                .glyph_sources()
                .iter()
                .map(|s| s.byte_range())
                .collect();
            assert_eq!(ranges, vec![0..1, 3..5, 6..7]);
            assert_eq!(
                mapped
                    .glyph_sources()
                    .iter()
                    .map(|s| s.run_index())
                    .collect::<Vec<_>>(),
                vec![0, 1, 2]
            );
        }
    }
}
