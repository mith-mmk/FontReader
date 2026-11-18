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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextDirection {
    #[default]
    LeftToRight,
    RightToLeft,
    TopToBottom,
}

impl TextDirection {
    pub(crate) fn is_vertical(self) -> bool {
        matches!(self, Self::TopToBottom)
    }

    pub(crate) fn is_right_to_left(self) -> bool {
        matches!(self, Self::RightToLeft)
    }
}

/// Font-level metrics. Keep this on the glyph so mixed fallback fonts can coexist in one run.
#[derive(Debug, Clone, Copy)]
pub struct FontMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
    pub flow: GlyphFlow,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

/// Paint for vector glyph layers.
///
/// `Solid(u32)` uses packed `0xAARRGGBB`, which matches `paintcore::path::draw_glyphs`.
/// `CurrentColor` maps to the default color passed into the renderer.
#[derive(Debug, Clone, PartialEq)]
pub enum GlyphPaint {
    Solid(u32),
    CurrentColor,
    LinearGradient(GlyphLinearGradient),
    RadialGradient(GlyphRadialGradient),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlyphGradientStop {
    pub offset: f32,
    pub color: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphGradientSpread {
    Pad,
    Reflect,
    Repeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphGradientUnits {
    ObjectBoundingBox,
    UserSpaceOnUse,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlyphLinearGradient {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub units: GlyphGradientUnits,
    pub transform: [f32; 6],
    pub spread: GlyphGradientSpread,
    pub stops: Vec<GlyphGradientStop>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlyphRadialGradient {
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
    pub fx: f32,
    pub fy: f32,
    pub units: GlyphGradientUnits,
    pub transform: [f32; 6],
    pub spread: GlyphGradientSpread,
    pub stops: Vec<GlyphGradientStop>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPaintMode {
    Fill,
    Stroke,
}

/// Vector glyph layer.
///
/// This matches the layer contract documented in `doc/SVFONTSPEC.md`.
#[derive(Debug, Clone)]
pub struct PathGlyphLayer {
    pub commands: Vec<Command>,
    pub clip_commands: Vec<Command>,
    pub paint: GlyphPaint,
    pub paint_mode: PathPaintMode,
    pub fill_rule: FillRule,
    pub stroke_width: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

impl PathGlyphLayer {
    pub fn new(commands: Vec<Command>, paint: GlyphPaint) -> Self {
        Self {
            commands,
            clip_commands: Vec::new(),
            paint,
            paint_mode: PathPaintMode::Fill,
            fill_rule: FillRule::NonZero,
            stroke_width: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }

    pub fn stroke(commands: Vec<Command>, paint: GlyphPaint, stroke_width: f32) -> Self {
        Self {
            commands,
            clip_commands: Vec::new(),
            paint,
            paint_mode: PathPaintMode::Stroke,
            fill_rule: FillRule::NonZero,
            stroke_width,
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

/// Embedded SVG glyph payload extracted from an OpenType `SVG ` table.
///
/// `GlyphLayer::Svg` remains intentionally available as a raw payload fallback and for future
/// delegated renderers, even when a subset of SVG glyphs can already be converted into
/// `GlyphLayer::Path`.
///
/// The existing `view_box_*`, `width`, and `height` fields describe scaled
/// layout/bounds metadata; they are not the source SVG root's viewport. Use
/// [`SvgGlyphRun::svg_layer_transform`] for an explicit rendering coordinate
/// contract rather than inferring a scale from these fields or glyph advance.
#[cfg(feature = "svg-fonts")]
#[derive(Debug, Clone)]
pub struct SvgGlyphLayer {
    pub document: String,
    pub view_box_min_x: f32,
    pub view_box_min_y: f32,
    pub view_box_width: f32,
    pub view_box_height: f32,
    pub width: f32,
    pub height: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

/// The resolved glyphs and their SVG viewport-to-output transforms.
#[cfg(feature = "svg-fonts")]
#[derive(Debug, Clone)]
pub struct SvgGlyphRun {
    pub(crate) run: GlyphRun,
    pub(crate) placements: SvgPlacements,
}

#[cfg(feature = "svg-fonts")]
impl SvgGlyphRun {
    /// The shaped run, immutable so the SVG transforms stay paired.
    pub fn run(&self) -> &GlyphRun {
        &self.run
    }

    /// Consume the companion and return the shaped run without placement metadata.
    pub fn into_run(self) -> GlyphRun {
        self.run
    }

    /// Matrix `[a, b, c, d, e, f]` mapping the SVG root's viewport/design units
    /// to the shaped run's output coordinates: `x'=a*x+c*y+e`, `y'=b*x+d*y+f`.
    ///
    /// Apply SVG's own root `viewBox`/`preserveAspectRatio` mapping first, then
    /// this matrix. Root width/height are the extracted font viewport, not the
    /// glyph advance. Translation already includes glyph origin and layer
    /// offset; do not add either twice. The legacy `view_box_*`/`width`/`height`
    /// fields are layout metadata, not this coordinate transform.
    ///
    /// Returns None for invalid indices, non-SVG layers, or non-finite output.
    /// Zero horizontal stretch is preserved.
    pub fn svg_layer_transform(&self, glyph_index: usize, layer_index: usize) -> Option<[f32; 6]> {
        self.placements.get(glyph_index, layer_index)
    }
}

#[cfg(feature = "svg-fonts")]
#[derive(Debug, Clone, Default)]
pub(crate) struct SvgPlacements {
    // Only raw SVG layers take storage. Appends follow glyph/layer order.
    entries: Vec<(usize, usize, [f32; 6])>,
}

#[cfg(feature = "svg-fonts")]
impl SvgPlacements {
    pub(crate) fn append_run(
        &mut self,
        font: &crate::fontreader::Font,
        options: &FontOptions<'_>,
        glyphs: &[PositionedGlyph],
        start: usize,
        turned: bool,
    ) -> Result<(), Error> {
        let mut scale = None;
        for (glyph_index, glyph) in glyphs.iter().enumerate() {
            for (layer_index, layer) in glyph.glyph.layers.iter().enumerate() {
                let GlyphLayer::Svg(layer) = layer else {
                    continue;
                };
                let (sx, sy) = match scale {
                    Some(scales) => scales,
                    None => {
                        let scales = font.glyph_scale_with_options(options)?;
                        scale = Some(scales);
                        scales
                    }
                };
                self.append(
                    start + glyph_index,
                    layer_index,
                    glyph,
                    layer,
                    sx,
                    sy,
                    turned,
                );
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn append(
        &mut self,
        glyph_index: usize,
        layer_index: usize,
        glyph: &PositionedGlyph,
        layer: &SvgGlyphLayer,
        sx: f32,
        sy: f32,
        turned: bool,
    ) {
        let (x, y) = (glyph.x + layer.offset_x, glyph.y + layer.offset_y);
        let matrix = if turned {
            [0.0, sx, -sy, 0.0, x, y]
        } else {
            [sx, 0.0, 0.0, sy, x, y]
        };
        if matrix.iter().all(|value| value.is_finite()) {
            self.entries.push((glyph_index, layer_index, matrix));
        }
    }

    pub(crate) fn get(&self, glyph: usize, layer: usize) -> Option<[f32; 6]> {
        let index = self
            .entries
            .binary_search_by_key(&(glyph, layer), |entry| (entry.0, entry.1))
            .ok()?;
        Some(self.entries[index].2)
    }
}

#[cfg(all(test, feature = "svg-fonts"))]
#[test]
fn svg_context_ignores_payload_metadata_and_legacy_layout_box() {
    let svg = SvgGlyphLayer {
        document: "<svg data-fontcore-svg-scale-x='999' viewBox='0 -900 80 80'/>".into(),
        view_box_min_x: -100.0,
        view_box_min_y: -800.0,
        view_box_width: 500.0,
        view_box_height: 700.0,
        width: 123.0,
        height: 456.0,
        offset_x: 3.0,
        offset_y: -4.0,
    };
    let run = GlyphRun::new(vec![PositionedGlyph::new(
        Glyph::new(vec![GlyphLayer::Svg(svg)]),
        20.0,
        30.0,
    )]);
    let glyph = &run.glyphs[0];
    let GlyphLayer::Svg(svg) = &glyph.glyph.layers[0] else {
        unreachable!()
    };
    let mut placements = SvgPlacements::default();
    placements.append(0, 0, glyph, svg, 0.2, 0.1, false);
    placements.append(0, 1, glyph, svg, f32::INFINITY, 0.1, false);
    let context = SvgGlyphRun { run, placements };
    assert_eq!(
        context.svg_layer_transform(0, 0),
        Some([0.2, 0.0, 0.0, 0.1, 23.0, 26.0])
    );
    assert_eq!(context.svg_layer_transform(0, 1), None);
}

/// Extensible glyph layer model.
#[derive(Debug, Clone)]
pub enum GlyphLayer {
    Path(PathGlyphLayer),
    Raster(RasterGlyphLayer),
    #[cfg(feature = "svg-fonts")]
    Svg(SvgGlyphLayer),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

/// GSUB variant selection exposed through the public API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontVariant {
    #[default]
    Normal,
    SmallCaps,
    Jis78,
    Jis90,
    TraditionalForms,
    NlcKanjiForms,
}

impl FontVariant {
    #[cfg_attr(not(feature = "layout"), allow(dead_code))]
    pub(crate) fn gsub_feature_tags(self) -> &'static [[u8; 4]] {
        const EMPTY: &[[u8; 4]] = &[];
        const SMALL_CAPS: &[[u8; 4]] = &[*b"c2sc", *b"smcp"];
        const JP78: &[[u8; 4]] = &[*b"jp78"];
        const JP90: &[[u8; 4]] = &[*b"jp90"];
        const TRAD: &[[u8; 4]] = &[*b"trad"];
        const NLCK: &[[u8; 4]] = &[*b"nlck"];

        match self {
            Self::Normal => EMPTY,
            Self::SmallCaps => SMALL_CAPS,
            Self::Jis78 => JP78,
            Self::Jis90 => JP90,
            Self::TraditionalForms => TRAD,
            Self::NlcKanjiForms => NLCK,
        }
    }
}

/// One variable-font axis value such as `wght=700` or `wdth=75`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontVariationSetting {
    pub tag: [u8; 4],
    pub value: f32,
}

impl FontVariationSetting {
    /// Creates one axis setting from a four-character OpenType tag.
    pub fn new(tag: &str, value: f32) -> Result<Self, Error> {
        Ok(Self {
            tag: parse_variation_tag(tag)?,
            value,
        })
    }

    /// Returns the OpenType tag as a string such as `"wght"`.
    pub fn tag_string(&self) -> String {
        String::from_utf8_lossy(&self.tag).into_owned()
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

#[derive(Clone)]
pub enum FontRef<'a> {
    Loaded(&'a crate::FontFace),
    #[cfg(feature = "raw")]
    Parsed(&'a crate::fontreader::Font),
    Family(&'a crate::FontFamily),
}

/// High-level options used by shaping, measurement, and SVG export.
#[derive(Clone)]
pub struct FontOptions<'a> {
    pub font: Option<FontRef<'a>>,
    pub font_family: Option<&'a str>,
    pub font_name: Option<&'a str>,
    pub locale: Option<&'a str>,
    pub text_direction: TextDirection,
    pub font_size: f32,
    pub font_stretch: FontStretch,
    pub font_style: FontStyle,
    pub font_variant: FontVariant,
    pub font_weight: FontWeight,
    pub line_height: Option<f32>,
    pub variations: Vec<FontVariationSetting>,
}

impl<'a> FontOptions<'a> {
    pub fn new(font: &'a crate::FontFace) -> Self {
        Self::from_font_ref(FontRef::Loaded(font))
    }

    pub fn from_family(font_family: &'a crate::FontFamily) -> Self {
        Self::from_font_ref(FontRef::Family(font_family))
    }

    #[cfg(feature = "raw")]
    pub fn from_font(font: &'a crate::fontreader::Font) -> Self {
        Self::from_parsed(font)
    }

    pub(crate) fn from_parsed(font: &'a crate::fontreader::Font) -> Self {
        let _ = font;
        #[cfg(feature = "raw")]
        {
            Self::from_font_ref(FontRef::Parsed(font))
        }
        #[cfg(not(feature = "raw"))]
        {
            Self {
                font: None,
                font_family: None,
                font_name: None,
                locale: None,
                text_direction: TextDirection::default(),
                font_size: 16.0,
                font_stretch: FontStretch::default(),
                font_style: FontStyle::default(),
                font_variant: FontVariant::default(),
                font_weight: FontWeight::default(),
                line_height: None,
                variations: Vec::new(),
            }
        }
    }

    pub fn from_font_ref(font: FontRef<'a>) -> Self {
        Self {
            font: Some(font),
            font_family: None,
            font_name: None,
            locale: None,
            text_direction: TextDirection::default(),
            font_size: 16.0,
            font_stretch: FontStretch::default(),
            font_style: FontStyle::default(),
            font_variant: FontVariant::default(),
            font_weight: FontWeight::default(),
            line_height: None,
            variations: Vec::new(),
        }
    }

    pub fn with_loaded_font(mut self, font: &'a crate::FontFace) -> Self {
        self.font = Some(FontRef::Loaded(font));
        self
    }

    pub fn with_face(self, font: &'a crate::FontFace) -> Self {
        self.with_loaded_font(font)
    }

    #[cfg(feature = "raw")]
    pub fn with_font(mut self, font: &'a crate::fontreader::Font) -> Self {
        self.font = Some(FontRef::Parsed(font));
        self
    }

    pub fn with_family(mut self, font_family: &'a crate::FontFamily) -> Self {
        self.font = Some(FontRef::Family(font_family));
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

    pub fn with_font_stretch(mut self, font_stretch: FontStretch) -> Self {
        self.font_stretch = font_stretch;
        self
    }

    pub fn with_font_style(mut self, font_style: FontStyle) -> Self {
        self.font_style = font_style;
        self
    }

    pub fn with_font_variant(mut self, font_variant: FontVariant) -> Self {
        self.font_variant = font_variant;
        self
    }

    pub fn with_font_weight(mut self, font_weight: FontWeight) -> Self {
        self.font_weight = font_weight;
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

    pub fn with_locale(mut self, locale: &'a str) -> Self {
        self.locale = Some(locale);
        self
    }

    pub fn with_text_direction(mut self, text_direction: TextDirection) -> Self {
        self.text_direction = text_direction;
        self
    }

    pub fn with_variation(mut self, tag: &str, value: f32) -> Self {
        if let Ok(setting) = FontVariationSetting::new(tag, value) {
            if let Some(existing) = self
                .variations
                .iter_mut()
                .find(|existing| existing.tag == setting.tag)
            {
                existing.value = value;
            } else {
                self.variations.push(setting);
            }
        }
        self
    }

    pub fn with_variations(mut self, variations: &[FontVariationSetting]) -> Self {
        self.variations = variations.to_vec();
        self
    }

    pub fn clear_variations(mut self) -> Self {
        self.variations.clear();
        self
    }

    pub fn with_vertical_flow(self) -> Self {
        self.with_text_direction(TextDirection::TopToBottom)
    }

    pub fn with_right_to_left(self) -> Self {
        self.with_text_direction(TextDirection::RightToLeft)
    }

    pub fn resolve_font(&self) -> Result<&'a crate::fontreader::Font, Error> {
        if let Some(font) = &self.font {
            return match font {
                FontRef::Loaded(font) => Ok(font.font()),
                #[cfg(feature = "raw")]
                FontRef::Parsed(font) => Ok(font),
                FontRef::Family(font_family) => Ok(font_family.resolve_font_options(self)?.font()),
            };
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
    if let Some(FontRef::Family(font_family)) = options.font {
        return font_family.text2glyph_run(text, options);
    }
    let font = options.resolve_font()?;
    font.text2glyph_run(text, &options)
}

pub(crate) fn parse_variation_tag(tag: &str) -> Result<[u8; 4], Error> {
    let bytes = tag.as_bytes();
    if bytes.len() != 4 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("variation tag must be 4 ASCII bytes, got {tag:?}"),
        ));
    }
    let mut result = [0u8; 4];
    result.copy_from_slice(bytes);
    Ok(result)
}
