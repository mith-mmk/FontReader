pub mod fontheader;
pub mod fontreader;
pub mod opentype;
pub(crate) mod util;
pub type Font = fontreader::Font;
pub use fontreader::{GlyphCommands, PathCommand};
pub mod commands;
pub use commands as commads;
pub use commands::{
    text2commands, Command, FillRule, FontMetrics, FontOptions, FontRef, FontStretch, FontStyle,
    FontVariant, FontWeight, Glyph, GlyphBounds, GlyphFlow, GlyphLayer, GlyphMetrics, GlyphPaint,
    GlyphRun, PathGlyphLayer, PositionedGlyph, RasterGlyphLayer, RasterGlyphSource,
};
#[cfg(test)]
mod test;
pub mod woff;


use std::io::Error;
use std::io::ErrorKind;
use std::collections::HashMap;
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

    pub fn add_face(
        &mut self,
        descriptor: FontFaceDescriptor,
        font: LoadedFont,
    ) -> &LoadedFont {
        self.faces.push(CachedFontFace { descriptor, font });
        &self.faces.last().expect("face inserted").font
    }

    pub fn cached_descriptors(&self) -> Vec<&FontFaceDescriptor> {
        self.faces.iter().map(|face| &face.descriptor).collect()
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
        self.find_best_face(family_name, font_name, font_weight, font_style, font_stretch)
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
        self.find_best_face(family_name, font_name, font_weight, font_style, font_stretch)
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

                Some((face_match_score(descriptor, font_weight, font_style, font_stretch), face))
            })
            .min_by_key(|(score, _)| *score)
            .map(|(_, face)| face)
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
    let weight_delta = descriptor
        .font_weight
        .0
        .abs_diff(font_weight.0) as u32;
    let style_penalty = if descriptor.font_style == font_style {
        0
    } else {
        10_000
    };
    let stretch_delta = ((descriptor.font_stretch.0 - font_stretch.0).abs() * 1000.0) as u32;
    style_penalty + weight_delta + stretch_delta
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
    /// This returns `Vec<GlyphCommands>` and does not preserve per-layer paint or raster glyphs.
    pub fn text2commands(&self, text: &str) -> Result<Vec<GlyphCommands>, Error> {
        self.font.text2commands(text)
    }

    /// Legacy outline-only API.
    ///
    /// This is an alias of `LoadedFont::text2commands()` and does not preserve color glyph data.
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

pub fn fontload_file(path: impl AsRef<Path>) -> Result<LoadedFont, Error> {
    let font = fontreader::Font::get_font_from_file(&path.as_ref().to_path_buf())?;
    Ok(LoadedFont { font })
}

pub fn load_font_from_file(path: impl AsRef<Path>) -> Result<LoadedFont, Error> {
    fontload_file(path)
}

pub fn fontload_buffer(buffer: &[u8]) -> Result<LoadedFont, Error> {
    let font = fontreader::Font::get_font_from_buffer(buffer)?;
    Ok(LoadedFont { font })
}

pub fn load_font_from_buffer(buffer: &[u8]) -> Result<LoadedFont, Error> {
    fontload_buffer(buffer)
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
        fontload_buffer(&bytes)
    }
}

pub fn fontload_net(url: &str) -> Result<LoadedFont, Error> {
    load_font_from_net(url)
}

pub fn fontload(source: FontSource<'_>) -> Result<LoadedFont, Error> {
    match source {
        FontSource::File(path) => fontload_file(path),
        FontSource::Buffer(buffer) => fontload_buffer(buffer),
    }
}

pub fn load_font(source: FontSource<'_>) -> Result<LoadedFont, Error> {
    fontload(source)
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
            let port = port.parse::<u16>().map_err(|_| {
                Error::new(ErrorKind::InvalidInput, "invalid port in http URL")
            })?;
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
            format!("unexpected http status: {}", header.lines().next().unwrap_or("")),
        ));
    }

    Ok(response[header_end..].to_vec())
}
