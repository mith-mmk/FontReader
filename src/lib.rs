pub mod fontheader;
pub mod fontreader;
pub mod opentype;
pub(crate) mod util;
pub type Font = fontreader::Font;
pub mod woff;
#[cfg(test)]
mod test;