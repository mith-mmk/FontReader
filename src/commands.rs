use std::io::{Error, ErrorKind};

#[derive(Debug, Clone)]
pub enum Command {
    Line(f32, f32),
    MoveTo(f32, f32),
    Bezier((f32, f32), (f32, f32)),
    CubicBezier((f32, f32), (f32, f32), (f32, f32)),
    Close,
}

/// Text advance direction resolved by the font parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphFlow {
    Horizontal,
    Vertical,
}

/// Font-level metrics. Keep this on the glyph so mixed fallback fonts can coexist in one run.
#[derive(Debug, Clone, Copy)]
pub struct FontMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
    pub flow: GlyphFlow,
}

#[derive(Debug, Clone, Copy)]
pub struct GlyphBounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

/// Glyph metrics after the font parser has resolved units and orientation.
#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    pub advance_x: f32,
    pub advance_y: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub bounds: Option<GlyphBounds>,
}

impl Default for GlyphMetrics {
    fn default() -> Self {
        Self {
            advance_x: 0.0,
            advance_y: 0.0,
            bearing_x: 0.0,
            bearing_y: 0.0,
            bounds: None,
        }
    }
}

/// Paint for vector glyph layers. `CurrentColor` maps to the color passed into `draw_glyphs`.
#[derive(Debug, Clone, Copy)]
pub enum GlyphPaint {
    Solid(u32),
    CurrentColor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

/// Vector glyph layer.
#[derive(Debug, Clone)]
pub struct PathGlyphLayer {
    pub commands: Vec<Command>,
    pub paint: GlyphPaint,
    pub fill_rule: FillRule,
    pub offset_x: f32,
    pub offset_y: f32,
}

impl PathGlyphLayer {
    pub fn new(commands: Vec<Command>, paint: GlyphPaint) -> Self {
        Self {
            commands,
            paint,
            fill_rule: FillRule::NonZero,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

/// Raster glyph payload.
#[derive(Debug, Clone)]
pub enum RasterGlyphSource {
    Encoded(Vec<u8>),
    Rgba {
        width: u32,
        height: u32,
        data: Vec<u8>,
    },
}

#[derive(Debug, Clone)]
pub struct RasterGlyphLayer {
    pub source: RasterGlyphSource,
    pub offset_x: f32,
    pub offset_y: f32,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl RasterGlyphLayer {
    pub fn from_encoded(data: Vec<u8>) -> Self {
        Self {
            source: RasterGlyphSource::Encoded(data),
            offset_x: 0.0,
            offset_y: 0.0,
            width: None,
            height: None,
        }
    }

    pub fn from_rgba(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            source: RasterGlyphSource::Rgba {
                width,
                height,
                data,
            },
            offset_x: 0.0,
            offset_y: 0.0,
            width: None,
            height: None,
        }
    }
}

/// Extensible glyph layer model.
#[derive(Debug, Clone)]
pub enum GlyphLayer {
    Path(PathGlyphLayer),
    Raster(RasterGlyphLayer),
}

/// A single glyph as prepared by the font parser.
#[derive(Debug, Clone)]
pub struct Glyph {
    pub font: Option<FontMetrics>,
    pub metrics: GlyphMetrics,
    pub layers: Vec<GlyphLayer>,
}

impl Glyph {
    pub fn new(layers: Vec<GlyphLayer>) -> Self {
        Self {
            font: None,
            metrics: GlyphMetrics::default(),
            layers,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    pub glyph: Glyph,
    pub x: f32,
    pub y: f32,
}

impl PositionedGlyph {
    pub fn new(glyph: Glyph, x: f32, y: f32) -> Self {
        Self { glyph, x, y }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GlyphRun {
    pub glyphs: Vec<PositionedGlyph>,
}

impl GlyphRun {
    pub fn new(glyphs: Vec<PositionedGlyph>) -> Self {
        Self { glyphs }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontStretch(pub f32);

impl FontStretch {
    pub const NORMAL: Self = Self(1.0);
}

impl Default for FontStretch {
    fn default() -> Self {
        Self::NORMAL
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

impl Default for FontStyle {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontVariant {
    Normal,
    SmallCaps,
}

impl Default for FontVariant {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontWeight(pub u16);

impl FontWeight {
    pub const THIN: Self = Self(100);
    pub const EXTRA_LIGHT: Self = Self(200);
    pub const LIGHT: Self = Self(300);
    pub const NORMAL: Self = Self(400);
    pub const MEDIUM: Self = Self(500);
    pub const SEMI_BOLD: Self = Self(600);
    pub const BOLD: Self = Self(700);
    pub const EXTRA_BOLD: Self = Self(800);
    pub const BLACK: Self = Self(900);
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

#[derive(Clone, Copy)]
pub enum FontRef<'a> {
    Loaded(&'a crate::LoadedFont),
    Parsed(&'a crate::fontreader::Font),
}

impl<'a> FontRef<'a> {
    pub fn as_font(self) -> &'a crate::fontreader::Font {
        match self {
            Self::Loaded(font) => font.font(),
            Self::Parsed(font) => font,
        }
    }
}

#[derive(Clone, Copy)]
pub struct FontOptions<'a> {
    pub font: Option<FontRef<'a>>,
    pub font_family: Option<&'a str>,
    pub font_name: Option<&'a str>,
    pub font_size: f32,
    pub font_stretch: FontStretch,
    pub font_style: FontStyle,
    pub font_variant: FontVariant,
    pub font_weight: FontWeight,
    pub line_height: Option<f32>,
}

impl<'a> FontOptions<'a> {
    pub fn new(font: &'a crate::LoadedFont) -> Self {
        Self::from_font_ref(FontRef::Loaded(font))
    }

    pub fn from_font(font: &'a crate::fontreader::Font) -> Self {
        Self::from_font_ref(FontRef::Parsed(font))
    }

    pub fn from_font_ref(font: FontRef<'a>) -> Self {
        Self {
            font: Some(font),
            font_family: None,
            font_name: None,
            font_size: 16.0,
            font_stretch: FontStretch::default(),
            font_style: FontStyle::default(),
            font_variant: FontVariant::default(),
            font_weight: FontWeight::default(),
            line_height: None,
        }
    }

    pub fn with_loaded_font(mut self, font: &'a crate::LoadedFont) -> Self {
        self.font = Some(FontRef::Loaded(font));
        self
    }

    pub fn with_font(mut self, font: &'a crate::fontreader::Font) -> Self {
        self.font = Some(FontRef::Parsed(font));
        self
    }

    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    pub fn with_line_height(mut self, line_height: f32) -> Self {
        self.line_height = Some(line_height);
        self
    }

    pub fn with_font_name(mut self, font_name: &'a str) -> Self {
        self.font_name = Some(font_name);
        self
    }

    pub fn with_font_family(mut self, font_family: &'a str) -> Self {
        self.font_family = Some(font_family);
        self
    }

    pub fn resolve_font(&self) -> Result<&'a crate::fontreader::Font, Error> {
        if let Some(font) = self.font {
            return Ok(font.as_font());
        }

        if self.font_name.is_some() || self.font_family.is_some() {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "font lookup by name or family is not implemented yet; pass a loaded font in FontOptions::font",
            ));
        }

        Err(Error::new(
            ErrorKind::InvalidInput,
            "FontOptions requires a loaded font",
        ))
    }
}

pub fn text2commands(text: &str, options: FontOptions<'_>) -> Result<GlyphRun, Error> {
    let font = options.resolve_font()?;
    font.text2glyph_run(text, &options)
}
