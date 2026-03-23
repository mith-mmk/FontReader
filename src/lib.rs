pub mod fontheader;
pub mod fontreader;
pub mod opentype;
pub(crate) mod util;
pub type Font = fontreader::Font;
pub use fontreader::{GlyphCommands, PathCommand};
pub mod commads;
#[cfg(test)]
mod test;
pub mod woff;


use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
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

pub fn fontload_buffer(buffer: &[u8]) -> Result<LoadedFont, Error> {
    let font = fontreader::Font::get_font_from_buffer(buffer)?;
    Ok(LoadedFont { font })
}

pub fn load_font_from_net(url: &str) -> Result<LoadedFont, Error> {
    #[cfg(target_arch = "wasm32")]
    {
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
