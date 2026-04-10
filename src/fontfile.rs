//! File and collection entry points for the public API.

use crate::fontface::FontFace;
use crate::fontreader;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;
use std::io::{Error, ErrorKind};
#[cfg(not(target_arch = "wasm32"))]
use std::net::TcpStream;
use std::path::Path;

/// Source used by [`open_font`] and [`load_font`].
pub enum FontSource<'a> {
    /// Load from a filesystem path.
    File(&'a Path),
    /// Load from an in-memory byte slice.
    Buffer(&'a [u8]),
}

/// Owns a font file or collection and lets callers choose faces from it.
#[derive(Debug, Clone)]
pub struct FontFile {
    font: fontreader::Font,
}

impl FontFile {
    /// Opens a font file from disk.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        let font = fontreader::Font::get_font_from_file(&path.as_ref().to_path_buf())?;
        Ok(Self { font })
    }

    /// Opens a font from bytes already loaded in memory.
    pub fn from_buffer(buffer: &[u8]) -> Result<Self, Error> {
        let font = fontreader::Font::get_font_from_buffer(buffer)?;
        Ok(Self { font })
    }

    /// Opens a font from a generic [`FontSource`].
    pub fn from_source(source: FontSource<'_>) -> Result<Self, Error> {
        match source {
            FontSource::File(path) => Self::from_file(path),
            FontSource::Buffer(buffer) => Self::from_buffer(buffer),
        }
    }

    /// Opens a font over plain `http://`.
    ///
    /// This is not available on `wasm32`.
    pub fn from_net(url: &str) -> Result<Self, Error> {
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
            Self::from_buffer(&bytes)
        }
    }

    /// Returns the number of faces in the file or collection.
    pub fn face_count(&self) -> usize {
        self.font.get_font_count()
    }

    /// Returns one face by index.
    pub fn face(&self, index: usize) -> Result<FontFace, Error> {
        let mut font = self.font.clone();
        font.set_font(index)
            .map_err(|message| Error::new(ErrorKind::InvalidInput, message))?;
        Ok(FontFace::from_font(font))
    }

    /// Returns the currently selected face.
    pub fn current_face(&self) -> Result<FontFace, Error> {
        self.face(self.font.get_font_number())
    }

    /// Returns all faces in the file or collection.
    pub fn faces(&self) -> Result<Vec<FontFace>, Error> {
        let mut faces = Vec::with_capacity(self.face_count());
        for index in 0..self.face_count() {
            faces.push(self.face(index)?);
        }
        Ok(faces)
    }

    /// Dumps a small human-readable summary of the loaded file.
    pub fn dump(&self) -> String {
        format!(
            "FontFile\nface_count: {}\ncurrent_face: {}\nformat: {}",
            self.face_count(),
            self.font.get_font_number(),
            self.font.font_type.to_string()
        )
    }

    #[cfg(feature = "raw")]
    /// Returns the low-level parsed font when `raw` is enabled.
    pub fn raw_font(&self) -> &crate::fontreader::Font {
        &self.font
    }
}

/// Opens a [`FontFile`] from disk.
pub fn open_font_from_file(path: impl AsRef<Path>) -> Result<FontFile, Error> {
    FontFile::from_file(path)
}

/// Opens a [`FontFile`] from memory.
pub fn open_font_from_buffer(buffer: &[u8]) -> Result<FontFile, Error> {
    FontFile::from_buffer(buffer)
}

/// Opens a [`FontFile`] from plain `http://`.
pub fn open_font_from_net(url: &str) -> Result<FontFile, Error> {
    FontFile::from_net(url)
}

/// Opens a [`FontFile`] from a generic [`FontSource`].
pub fn open_font(source: FontSource<'_>) -> Result<FontFile, Error> {
    FontFile::from_source(source)
}

/// Loads the current face from a file.
pub fn load_font_from_file(path: impl AsRef<Path>) -> Result<FontFace, Error> {
    FontFile::from_file(path)?.current_face()
}

/// Loads the current face from an in-memory buffer.
pub fn load_font_from_buffer(buffer: &[u8]) -> Result<FontFace, Error> {
    FontFile::from_buffer(buffer)?.current_face()
}

/// Loads the current face from plain `http://`.
pub fn load_font_from_net(url: &str) -> Result<FontFace, Error> {
    FontFile::from_net(url)?.current_face()
}

/// Loads the current face from a generic [`FontSource`].
pub fn load_font(source: FontSource<'_>) -> Result<FontFace, Error> {
    FontFile::from_source(source)?.current_face()
}

#[cfg(feature = "raw")]
#[deprecated(note = "use `load_font_from_file()` instead")]
pub fn fontload_file(path: impl AsRef<Path>) -> Result<FontFace, Error> {
    load_font_from_file(path)
}

#[cfg(feature = "raw")]
#[deprecated(note = "use `load_font_from_buffer()` instead")]
pub fn fontload_buffer(buffer: &[u8]) -> Result<FontFace, Error> {
    load_font_from_buffer(buffer)
}

#[cfg(feature = "raw")]
#[deprecated(note = "use `load_font_from_net()` instead")]
pub fn fontload_net(url: &str) -> Result<FontFace, Error> {
    load_font_from_net(url)
}

#[cfg(feature = "raw")]
#[deprecated(note = "use `load_font()` instead")]
pub fn fontload(source: FontSource<'_>) -> Result<FontFace, Error> {
    load_font(source)
}

/// Collects a font file incrementally before decoding it.
pub struct ChunkedFontBuffer {
    total_size: usize,
    data: Vec<u8>,
    filled: Vec<bool>,
    filled_len: usize,
}

impl ChunkedFontBuffer {
    /// Creates an empty buffer for a known final font size.
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

    /// Returns the target total size in bytes.
    pub fn total_size(&self) -> usize {
        self.total_size
    }

    /// Returns how many bytes are already filled.
    pub fn filled_len(&self) -> usize {
        self.filled_len
    }

    /// Returns `true` when every byte has been supplied.
    pub fn is_complete(&self) -> bool {
        self.filled_len == self.total_size
    }

    /// Appends one chunk at the given byte offset.
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

    /// Returns missing byte ranges as half-open intervals.
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

    /// Clones the complete font bytes.
    pub fn to_vec(&self) -> Result<Vec<u8>, Error> {
        if !self.is_complete() {
            return Err(Error::new(
                ErrorKind::WouldBlock,
                "font buffer is incomplete; append all chunks before decoding",
            ));
        }

        Ok(self.data.clone())
    }

    /// Decodes the collected bytes as a [`FontFile`].
    pub fn load_font_file(&self) -> Result<FontFile, Error> {
        let bytes = self.to_vec()?;
        open_font_from_buffer(&bytes)
    }

    /// Decodes the collected bytes and returns the current face.
    pub fn load_font_face(&self) -> Result<FontFace, Error> {
        self.load_font_file()?.current_face()
    }

    #[cfg(feature = "raw")]
    pub fn load_font(&self) -> Result<FontFace, Error> {
        self.load_font_face()
    }

    /// Consumes the buffer and decodes it as a [`FontFile`].
    pub fn into_font_file(self) -> Result<FontFile, Error> {
        if !self.is_complete() {
            return Err(Error::new(
                ErrorKind::WouldBlock,
                "font buffer is incomplete; append all chunks before decoding",
            ));
        }

        open_font_from_buffer(&self.data)
    }

    /// Consumes the buffer and returns the current face.
    pub fn into_font_face(self) -> Result<FontFace, Error> {
        self.into_font_file()?.current_face()
    }

    #[cfg(feature = "raw")]
    pub fn into_loaded_font(self) -> Result<FontFace, Error> {
        self.into_font_face()
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
