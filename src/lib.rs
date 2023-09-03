pub mod fontheader;
pub mod fontreader;
pub mod opentype;

pub type Font = fontreader::Font;

#[cfg(test)]
mod test;