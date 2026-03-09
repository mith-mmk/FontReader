pub mod fontheader;
pub mod fontreader;
pub mod opentype;
pub(crate) mod util;
pub type Font = fontreader::Font;
pub use fontreader::{GlyphCommands, PathCommand};
#[cfg(test)]
mod test;
pub mod woff;

use std::io::Error;
use std::path::Path;

pub enum FontSource<'a> {
    File(&'a Path),
    Buffer(&'a [u8]),
}

pub struct LoadedFont {
    font: fontreader::Font,
}

impl LoadedFont {
    pub fn text2svg(&self, text: &str, fontsize: f64, fontunit: &str) -> Result<String, Error> {
        self.font.text2svg(text, fontsize, fontunit)
    }

    pub fn text2commands(&self, text: &str) -> Result<Vec<GlyphCommands>, Error> {
        self.font.text2commands(text)
    }

    pub fn font(&self) -> &fontreader::Font {
        &self.font
    }
}

pub fn fontload_file(path: impl AsRef<Path>) -> Result<LoadedFont, Error> {
    let font = fontreader::Font::get_font_from_file(&path.as_ref().to_path_buf())?;
    Ok(LoadedFont { font })
}

pub fn fontload_buffer(buffer: &[u8]) -> Result<LoadedFont, Error> {
    let font = fontreader::Font::get_font_from_buffer(buffer)?;
    Ok(LoadedFont { font })
}

pub fn fontload(source: FontSource<'_>) -> Result<LoadedFont, Error> {
    match source {
        FontSource::File(path) => fontload_file(path),
        FontSource::Buffer(buffer) => fontload_buffer(buffer),
    }
}
