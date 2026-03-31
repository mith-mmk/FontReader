#[cfg(feature = "raw")]
pub mod fontheader;
#[cfg(not(feature = "raw"))]
#[allow(dead_code)]
mod fontheader;

#[cfg(feature = "raw")]
pub mod fontreader;
#[cfg(not(feature = "raw"))]
#[allow(dead_code)]
mod fontreader;

#[cfg(feature = "raw")]
pub mod opentype;
#[cfg(not(feature = "raw"))]
#[allow(dead_code, unused_imports)]
mod opentype;

pub mod commands;
pub mod fontengine;
pub mod fontface;
pub mod fontfile;
pub(crate) mod util;
pub mod woff;

#[deprecated(note = "use `fontloader::commands` instead")]
pub use commands as commads;
pub use commands::{
    text2commands, Command, FillRule, FontMetrics, FontOptions, FontRef, FontStretch, FontStyle,
    FontVariant, FontWeight, Glyph, GlyphBounds, GlyphFlow, GlyphLayer, GlyphMetrics, GlyphPaint,
    GlyphRun, PathGlyphLayer, PositionedGlyph, RasterGlyphLayer, RasterGlyphSource, TextDirection,
};
pub use fontengine::{FontEngine, ShapingPolicy};
pub use fontface::{FontFace, FontFaceDescriptor, FontFamily};
pub use fontfile::{
    load_font, load_font_from_buffer, load_font_from_file, load_font_from_net, open_font,
    open_font_from_buffer, open_font_from_file, open_font_from_net, ChunkedFontBuffer, FontFile,
    FontSource,
};

#[cfg(feature = "raw")]
#[allow(deprecated)]
pub use fontfile::{fontload, fontload_buffer, fontload_file, fontload_net};

#[cfg(feature = "raw")]
pub type Font = fontreader::Font;

#[cfg(feature = "raw")]
#[deprecated(note = "use `FontFace` instead")]
pub type LoadedFont = FontFace;

#[cfg(feature = "raw")]
pub use fontreader::{BitmapGlyphCommands, BitmapGlyphFormat, GlyphCommands, PathCommand};

#[cfg(all(test, feature = "raw"))]
mod test;
