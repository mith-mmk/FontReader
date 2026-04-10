//! Public API for loading fonts, selecting faces, shaping text, and exporting SVG.
//!
//! The intended high-level flow is:
//!
//! 1. Load a file with [`FontFile`]
//! 2. Pick one face as a [`FontFace`]
//! 3. Shape or render through [`FontEngine`]
//!
//! ```no_run
//! use fontcore::FontFile;
//!
//! let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
//! let svg = face.engine().with_font_size(32.0).render_svg("Hello")?;
//! assert!(svg.contains("<svg"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Enable `features = ["raw"]` if you need the older low-level parser surface.

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
#[cfg(feature = "svg-fonts")]
pub(crate) mod svgparse;
pub(crate) mod util;
pub mod woff;

#[deprecated(note = "use `fontcore::commands` instead")]
pub use commands as commads;
#[cfg(feature = "svg-fonts")]
pub use commands::SvgGlyphLayer;
pub use commands::{
    text2commands, Command, FillRule, FontMetrics, FontOptions, FontRef, FontStretch, FontStyle,
    FontVariant, FontVariationSetting, FontWeight, Glyph, GlyphBounds, GlyphFlow,
    GlyphGradientSpread, GlyphGradientStop, GlyphGradientUnits, GlyphLayer, GlyphLinearGradient,
    GlyphMetrics, GlyphPaint, GlyphRadialGradient, GlyphRun, PathGlyphLayer, PathPaintMode,
    PositionedGlyph, RasterGlyphLayer, RasterGlyphSource, TextDirection,
};
pub use fontengine::{FontEngine, ShapingPolicy};
pub use fontface::{FontFace, FontFaceDescriptor, FontFamily, FontVariationAxis};
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
