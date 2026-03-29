use base64::{engine::general_purpose, Engine as _};
use bin_rs::reader::{BinaryReader, BytesReader};
use std::collections::HashMap;
use std::io::{Error, ErrorKind, SeekFrom};
use std::{fs::File, path::PathBuf};

use crate::commands::{
    Command as DrawCommand, FontMetrics as DrawFontMetrics, Glyph, GlyphBounds, GlyphFlow,
    GlyphLayer, GlyphMetrics as DrawGlyphMetrics, GlyphPaint, GlyphRun, PathGlyphLayer,
    PositionedGlyph, RasterGlyphLayer,
};
use crate::fontheader;
use crate::opentype::color::sbix;
use crate::opentype::color::svg;
use crate::opentype::color::{colr, cpal};
#[cfg(feature = "layout")]
use crate::opentype::extentions::gdef;
#[cfg(feature = "layout")]
use crate::opentype::extentions::gpos;
#[cfg(feature = "layout")]
use crate::opentype::extentions::gsub;
use crate::opentype::platforms::PlatformID;
use crate::opentype::requires::cmap::CmapEncodings;
use crate::opentype::requires::hhea::HHEA;
use crate::opentype::requires::hmtx::LongHorMetric;
use crate::opentype::requires::name::NameID;
use crate::opentype::requires::vhea::VHEA;
use crate::opentype::requires::vmtx::VerticalMetric;
use crate::opentype::requires::*;
use crate::opentype::{outline::*, OTFHeader};
use crate::util::sniff_encoded_image_dimensions;

#[cfg(debug_assertions)]
use std::io::{BufWriter, Write};

#[derive(Debug, Clone)]
pub enum PathCommand {
    MoveTo { x: f64, y: f64 },
    LineTo { x: f64, y: f64 },
    QuadTo { cx: f64, cy: f64, x: f64, y: f64 },
    ClosePath,
}

#[derive(Debug, Clone)]
pub enum BitmapGlyphFormat {
    Png,
    Jpeg,
}

#[derive(Debug, Clone)]
pub struct BitmapGlyphCommands {
    pub offset_x: f64,
    pub offset_y: f64,
    pub width: f64,
    pub height: f64,
    pub format: BitmapGlyphFormat,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct GlyphCommands {
    pub ch: char,
    pub glyph_id: usize,
    pub origin_x: f64,
    pub origin_y: f64,
    pub advance_width: f64,
    pub commands: Vec<PathCommand>,
    pub bitmap: Option<BitmapGlyphCommands>,
}

#[derive(Debug, Clone)]

pub enum GlyphFormat {
    OpenTypeGlyph,
    CFF,
    CFF2,
    SVG,
    Bitmap,
    Unknown,
}

#[derive(Debug, Clone)]

pub enum FontLayout {
    Horizontal(HorizontalLayout),
    Vertical(VerticalLayout),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct GriphData {
    glyph_id: usize,
    pub(crate) open_type_glyf: Option<OpenTypeGlyph>,
}

#[derive(Debug, Clone)]
pub struct OpenTypeGlyph {
    layout: FontLayout,
    glyph: FontData,
}

#[derive(Debug, Clone)]
pub enum FontData {
    Glyph(glyf::Glyph),
    CFF(Vec<u8>),
    CFF2(Vec<u8>),
    SVG(String),
    Bitmap(String, Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct Font {
    pub font_type: fontheader::FontHeaders,
    pub(crate) cmap: Option<CmapEncodings>, // must
    pub(crate) head: Option<head::HEAD>,    // must
    pub(crate) hhea: Option<hhea::HHEA>,    // must
    pub(crate) hmtx: Option<hmtx::HMTX>,    // must
    pub(crate) maxp: Option<maxp::MAXP>,    // must
    pub(crate) name: Option<name::NAME>,    // must
    pub(crate) name_table: Option<name::NameTable>,
    pub(crate) os2: Option<os2::OS2>,    // must
    pub(crate) post: Option<post::POST>, // must
    pub(crate) loca: Option<loca::LOCA>, // openType font, CFF/CFF2 none
    pub(crate) glyf: Option<glyf::GLYF>, // openType font, CFF/CFF2 none
    #[cfg(feature = "cff")]
    pub(crate) cff: Option<cff::CFF>, // CFF font, openType none
    pub(crate) colr: Option<colr::COLR>,
    pub(crate) cpal: Option<cpal::CPAL>,
    #[cfg(feature = "layout")]
    pub(crate) gdef: Option<gdef::GDEF>,
    #[cfg(feature = "layout")]
    pub(crate) gpos: Option<gpos::GPOS>,
    #[cfg(feature = "layout")]
    pub(crate) gsub: Option<gsub::GSUB>,
    pub(crate) svg: Option<svg::SVG>,
    pub(crate) sbix: Option<sbix::SBIX>,
    pub(crate) vhea: Option<vhea::VHEA>,
    pub(crate) vmtx: Option<vmtx::VMTX>,
    hmtx_pos: Option<Pointer>,
    vmtx_pos: Option<Pointer>,
    loca_pos: Option<Pointer>, // OpenType font, CFF/CFF2 none
    glyf_pos: Option<Pointer>, // OpenType font, CFF/CFF2 none
    sbix_pos: Option<Pointer>,
    pub(crate) more_fonts: Box<Vec<Font>>,
    current_font: usize,
}

#[derive(Debug, Clone, Copy)]
enum ResolvedTextUnit {
    Glyph(ResolvedGlyph),
    Newline,
    Tab,
}

#[derive(Debug, Clone)]
pub(crate) enum ParsedTextUnit {
    Glyph {
        text: String,
        ch: char,
        variation_selector: char,
    },
    Newline,
    Tab,
}

#[derive(Debug, Clone, Copy)]
struct ResolvedGlyph {
    ch: char,
    glyph_id: usize,
    prefer_color: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct GlyphPositionAdjustment {
    placement_x: f32,
    placement_y: f32,
    advance_x: f32,
    advance_y: f32,
}

impl Font {
    fn empty() -> Self {
        Self {
            font_type: fontheader::FontHeaders::Unknown,
            cmap: None,
            head: None,
            hhea: None,
            hmtx: None,
            maxp: None,
            name: None,
            name_table: None,
            os2: None,
            post: None,
            loca: None,
            glyf: None,
            #[cfg(feature = "cff")]
            cff: None,
            colr: None,
            cpal: None,
            #[cfg(feature = "layout")]
            gdef: None,
            #[cfg(feature = "layout")]
            gpos: None,
            #[cfg(feature = "layout")]
            gsub: None,
            sbix: None,
            svg: None,
            vhea: None,
            vmtx: None,
            hmtx_pos: None,
            vmtx_pos: None,
            loca_pos: None,
            glyf_pos: None,
            sbix_pos: None,
            more_fonts: Box::<Vec<Font>>::default(),
            current_font: 0,
        }
    }

    pub fn get_name_list(&self, locale: &String) -> HashMap<u16, String> {
        let name_table = if self.current_font == 0 {
            self.name_table.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1]
                .name_table
                .as_ref()
                .unwrap()
        };
        let platform_id = PlatformID::Windows;
        let name = name_table.get_name_list(locale, platform_id);
        if name.is_empty() {
            let platform_id = PlatformID::Macintosh;
            name_table.get_name_list(locale, platform_id)
        } else {
            name
        }
    }

    pub fn get_font_from_file(filename: &PathBuf) -> Result<Self, Error> {
        font_load_from_file(filename)
    }

    pub fn get_font_from_buffer(fontdata: &[u8]) -> Result<Self, Error> {
        let mut reader = BytesReader::new(fontdata);
        let font_type = fontheader::get_font_type(&mut reader)?;
        if let fontheader::FontHeaders::WOFF2(header) = font_type {
            let declared_length = header.length as usize;
            if declared_length > fontdata.len() {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    format!(
                        "WOFF2 buffer is shorter than declared length: {} < {}",
                        fontdata.len(),
                        declared_length
                    ),
                ));
            }
            let mut input = &fontdata[..declared_length];
            let ttf = woff2::decode::convert_woff2_to_ttf(&mut input).map_err(|err| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!("Failed to decode WOFF2 font: {err}"),
                )
            })?;
            return Self::get_font_from_buffer(&ttf);
        }

        reader.seek(SeekFrom::Start(0))?;
        font_load(&mut reader)
    }

    pub(crate) fn get_h_metrix(&self, id: usize) -> LongHorMetric {
        if self.current_font == 0 {
            self.hmtx.as_ref().unwrap().get_metrix(id)
        } else {
            self.more_fonts[self.current_font - 1]
                .hmtx
                .as_ref()
                .unwrap()
                .get_metrix(id)
        }
    }

    pub(crate) fn get_v_metrix(&self, id: usize) -> VerticalMetric {
        if self.current_font == 0 {
            self.vmtx.as_ref().unwrap().get_metrix(id)
        } else {
            self.more_fonts[self.current_font - 1]
                .vmtx
                .as_ref()
                .unwrap()
                .get_metrix(id)
        }
    }

    pub fn get_vertical_layout(&self, id: usize) -> Option<VerticalLayout> {
        let vhea = if self.current_font == 0 {
            self.vhea.as_ref()
        } else {
            self.more_fonts[self.current_font - 1].vhea.as_ref()
        };

        if let Some(vhea) = vhea {
            let v_metrix = self.get_v_metrix(id);
            return Some(VerticalLayout {
                tsb: v_metrix.top_side_bearing as isize,
                advance_height: v_metrix.advance_height as isize,
                accender: vhea.get_accender() as isize,
                descender: vhea.get_descender() as isize,
                line_gap: vhea.get_line_gap() as isize,
                vhea: vhea.clone(),
            });
        } else {
            return None;
        }
    }

    pub fn get_horizontal_layout(&self, id: usize) -> HorizontalLayout {
        let h_metrix = self.get_h_metrix(id);

        let hhea = if self.current_font == 0 {
            self.hhea.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1]
                .hhea
                .as_ref()
                .unwrap()
        };

        let lsb = h_metrix.left_side_bearing as isize;
        let advance_width = h_metrix.advance_width as isize;

        let accender = hhea.get_accender() as isize;
        let descender = hhea.get_descender() as isize;
        let line_gap = hhea.get_line_gap() as isize;

        HorizontalLayout {
            lsb,
            advance_width,
            accender,
            descender,
            line_gap,
            hhea: hhea.clone(),
        }
    }

    pub fn get_glyph_from_id(&self, glyph_id: usize) -> GriphData {
        self.get_glyph_from_id_axis(glyph_id, false)
    }

    pub fn get_layout(&self, glyph_id: usize, is_vert: bool) -> FontLayout {
        let layout = if is_vert {
            let result = self.get_vertical_layout(glyph_id as usize);
            if result.is_some() {
                FontLayout::Vertical(result.unwrap())
            } else {
                FontLayout::Horizontal(self.get_horizontal_layout(glyph_id as usize))
            }
        } else {
            FontLayout::Horizontal(self.get_horizontal_layout(glyph_id as usize))
        };
        layout
    }

    pub fn get_glyph_with_uvs_axis(&self, ch: char, vs: char, is_vert: bool) -> GriphData {
        let glyph_id = self.resolve_glyph_id_with_uvs(ch, vs, is_vert).unwrap();
        self.get_glyph_from_id_axis(glyph_id, is_vert)
    }

    pub fn get_glyph_with_uvs(&self, ch: char, vs: char) -> GriphData {
        self.get_glyph_with_uvs_axis(ch, vs, false)
    }

    pub fn get_glyph(&self, ch: char) -> GriphData {
        self.get_glyph_with_uvs(ch, '\u{0}')
    }

    pub fn get_svg_from_id(
        &self,
        glyph_id: usize,
        fontsize: f64,
        fontunit: &str,
    ) -> Result<String, Error> {
        let layout = self.get_layout(glyph_id, false);
        #[cfg(feature = "cff")]
        if let Some(cff) = self.cff.as_ref() {
            let string = cff.to_svg(glyph_id, fontsize, fontunit, &layout, 0.0, 0.0)?;
            return Ok(string);
        }

        // utf-32
        let pos = glyph_id as u32;
        if let Some(glyf) = &self.glyf {
            let glyph = glyf.get_glyph(pos as usize);
            if glyph.is_none() {
                return Err(Error::new(
                    std::io::ErrorKind::Other,
                    "glyph is none,also you need --features cff".to_string(),
                ));
            }
            let glyph = glyph.unwrap();
            if let Some(sbix) = self.sbix.as_ref() {
                let result = sbix.get_svg(pos as u32, fontsize, fontunit, &layout, 0.0, 0.0);
                if let Some(svg) = result {
                    let mut string = "".to_string();
                    #[cfg(debug_assertions)]
                    {
                        string += &format!("<!-- glyf id: {} -->", pos);
                    }
                    string += &svg;
                    return Ok(string);
                }
            } else if let Some(svg) = self.svg.as_ref() {
                let result = svg.get_svg(pos as u32, fontsize, fontunit, &layout, 0.0, 0.0);
                if let Some(svg) = result {
                    let mut string = "".to_string();
                    #[cfg(debug_assertions)]
                    {
                        string += &format!("<!-- glyf id: {} -->", pos);
                        if let FontLayout::Horizontal(layout) = &layout {
                            string += &format!(
                                "<!-- layout {} {} {} {} {} -->\n",
                                layout.lsb,
                                layout.advance_width,
                                layout.accender,
                                layout.descender,
                                layout.line_gap
                            );
                        }
                    }
                    string += &svg;
                    return Ok(string);
                }
            }
            let glyf = if self.current_font == 0 {
                self.glyf.as_ref().unwrap()
            } else {
                self.more_fonts[self.current_font - 1]
                    .glyf
                    .as_ref()
                    .unwrap()
            };

            let (cpal, colr) = if self.current_font == 0 {
                (self.cpal.as_ref(), self.colr.as_ref())
            } else {
                (
                    self.more_fonts[self.current_font - 1].cpal.as_ref(),
                    self.more_fonts[self.current_font - 1].colr.as_ref(),
                )
            };

            if let Some(colr) = colr.as_ref() {
                let layers = colr.get_layer_record(pos as u16);
                if layers.is_empty() {
                    return Ok(glyf.to_svg(glyph_id, fontsize, fontunit, &layout, 0.0, 0.0));
                }
                let mut string = glyph.get_svg_heder(fontsize, fontunit, &layout);
                #[cfg(debug_assertions)]
                {
                    string += &format!("\n<!-- glyf id: {} -->", pos);
                }

                for layer in layers {
                    let glyf_id = layer.glyph_id as u32;
                    let pallet = cpal
                        .as_ref()
                        .unwrap()
                        .get_pallet(layer.palette_index as usize);
                    #[cfg(debug_assertions)]
                    {
                        string += &format!("<!-- pallet index {} -->\n", layer.palette_index);
                        string += &format!(
                            "<!-- Red {} Green {} Blue {} Alpha {} -->\n",
                            pallet.red, pallet.green, pallet.blue, pallet.alpha
                        );
                    }
                    string += &format!(
                        "<g fill=\"rgba({}, {}, {}, {})\">\n",
                        pallet.red, pallet.green, pallet.blue, pallet.alpha
                    );
                    string += &glyf.get_svg_path(glyf_id as usize, &layout, 0.0, 0.0);
                    string += "</g>\n";
                }
                string += "</svg>";
                Ok(string)
            } else {
                #[cfg(debug_assertions)]
                {
                    let string = glyf.to_svg(glyph_id, fontsize, fontunit, &layout, 0.0, 0.0);
                    return Ok(format!("<!-- glyf id: {} -->{}", pos, string));
                }
                #[cfg(not(debug_assertions))]
                Ok(glyf.to_svg(glyph_id, fontsize, fontunit, &layout, 0.0, 0.0))
            }
        } else {
            return Err(Error::new(
                std::io::ErrorKind::Other,
                "glyf is none".to_string(),
            ));
        }
    }

    pub fn get_svg_with_uvs_axis(
        &self,
        ch: char,
        vs: char,
        fontsize: f64,
        fontunit: &str,
        is_vert: bool,
    ) -> Result<String, Error> {
        // svg ?
        // sbix ?
        // cff ?

        #[cfg(feature = "cff")]
        if let Some(cff) = self.cff.as_ref() {
            let glyf_data = self.get_glyph_with_uvs_axis(ch, vs, is_vert);
            let glyph_id = glyf_data.glyph_id;
            let layout = self.get_layout(glyph_id as usize, is_vert);
            let string = cff.to_svg(glyph_id, fontsize, fontunit, &layout, 0.0, 0.0);
            return string;
        }

        if self.glyf.is_none() {
            return Err(Error::new(
                std::io::ErrorKind::Other,
                "glyf is none".to_string(),
            ));
        }

        // utf-32
        let glyf_data = self.get_glyph_with_uvs_axis(ch, vs, is_vert);
        let glyph_id = glyf_data.glyph_id;

        if let FontData::Glyph(glyph) = &glyf_data.open_type_glyf.as_ref().unwrap().glyph {
            let layout = &glyf_data.open_type_glyf.as_ref().unwrap().layout;
            if let Some(sbix) = self.sbix.as_ref() {
                let result = sbix.get_svg(glyph_id as u32, fontsize, fontunit, &layout, 0.0, 0.0);
                if let Some(svg) = result {
                    let mut string = "".to_string();
                    #[cfg(debug_assertions)]
                    {
                        string += &format!("<!-- {} glyf id: {} -->", ch, glyph_id);
                    }
                    string += &svg;
                    return Ok(string);
                }
            } else if let Some(svg) = self.svg.as_ref() {
                let result = svg.get_svg(glyph_id as u32, fontsize, fontunit, &layout, 0.0, 0.0);
                if let Some(svg) = result {
                    let mut string = "".to_string();
                    #[cfg(debug_assertions)]
                    {
                        string += &format!("<!-- {} glyf id: {} -->", ch, glyph_id);
                        if let FontLayout::Horizontal(layout) = layout {
                            string += &format!(
                                "<!-- layout {} {} {} {} {} -->\n",
                                layout.lsb,
                                layout.advance_width,
                                layout.accender,
                                layout.descender,
                                layout.line_gap
                            );
                        } else if let FontLayout::Vertical(layout) = layout {
                            string += &format!(
                                "<!-- layout vert {} {} {} {} {} -->\n",
                                layout.tsb,
                                layout.advance_height,
                                layout.accender,
                                layout.descender,
                                layout.line_gap
                            );
                        }
                    }
                    string += &svg;
                    return Ok(string);
                }
            }
            let glyf = if self.current_font == 0 {
                self.glyf.as_ref().unwrap()
            } else {
                self.more_fonts[self.current_font - 1]
                    .glyf
                    .as_ref()
                    .unwrap()
            };

            let (cpal, colr) = if self.current_font == 0 {
                (self.cpal.as_ref(), self.colr.as_ref())
            } else {
                (
                    self.more_fonts[self.current_font - 1].cpal.as_ref(),
                    self.more_fonts[self.current_font - 1].colr.as_ref(),
                )
            };

            if let Some(colr) = colr.as_ref() {
                let layers = colr.get_layer_record(glyph_id as u16);
                if layers.is_empty() {
                    return Ok(glyf.to_svg(glyph_id, fontsize, fontunit, &layout, 0.0, 0.0));
                }
                let mut string = glyph.get_svg_heder(fontsize, fontunit, &layout);
                #[cfg(debug_assertions)]
                {
                    string += &format!("\n<!-- {} glyf id: {} -->", ch, glyph_id);
                }

                for layer in layers {
                    let glyf_id = layer.glyph_id as u32;
                    let pallet = cpal
                        .as_ref()
                        .unwrap()
                        .get_pallet(layer.palette_index as usize);
                    #[cfg(debug_assertions)]
                    {
                        string += &format!("<!-- pallet index {} -->\n", layer.palette_index);
                        string += &format!(
                            "<!-- Red {} Green {} Blue {} Alpha {} -->\n",
                            pallet.red, pallet.green, pallet.blue, pallet.alpha
                        );
                    }
                    string += &format!(
                        "<g fill=\"rgba({}, {}, {}, {})\">\n",
                        pallet.red, pallet.green, pallet.blue, pallet.alpha
                    );
                    string += &glyf.get_svg_path(glyf_id as usize, &layout, 0.0, 0.0);
                    string += "</g>\n";
                }
                string += "</svg>";
                Ok(string)
            } else {
                #[cfg(debug_assertions)]
                {
                    let string = glyf.to_svg(glyph_id, fontsize, fontunit, &layout, 0.0, 0.0);
                    return Ok(format!("<!-- {} glyf id: {} -->{}", ch, glyph_id, string));
                }
                #[cfg(not(debug_assertions))]
                Ok(glyf.to_svg(glyph_id, fontsize, fontunit, &layout, 0.0, 0.0))
            }
        } else {
            return Err(Error::new(
                std::io::ErrorKind::Other,
                "glyf is none".to_string(),
            ));
        }
    }

    pub fn get_svg_with_uvs(
        &self,
        ch: char,
        vs: char,
        fontsize: f64,
        fontunit: &str,
    ) -> Result<String, Error> {
        self.get_svg_with_uvs_axis(ch, vs, fontsize, fontunit, false)
    }

    pub fn get_svg(&self, ch: char, fontsize: f64, fontunit: &str) -> Result<String, Error> {
        self.get_svg_with_uvs(ch, '\u{0}', fontsize, fontunit)
    }

    fn current_hhea(&self) -> Result<&HHEA, Error> {
        if self.current_font == 0 {
            self.hhea
                .as_ref()
                .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "hhea is none"))
        } else {
            self.more_fonts[self.current_font - 1]
                .hhea
                .as_ref()
                .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "hhea is none"))
        }
    }

    fn current_head(&self) -> Result<&head::HEAD, Error> {
        if self.current_font == 0 {
            self.head
                .as_ref()
                .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "head is none"))
        } else {
            self.more_fonts[self.current_font - 1]
                .head
                .as_ref()
                .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "head is none"))
        }
    }

    fn current_cmap(&self) -> Result<&CmapEncodings, Error> {
        if self.current_font == 0 {
            self.cmap
                .as_ref()
                .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "cmap is none"))
        } else {
            self.more_fonts[self.current_font - 1]
                .cmap
                .as_ref()
                .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "cmap is none"))
        }
    }

    fn current_glyf(&self) -> Option<&glyf::GLYF> {
        if self.current_font == 0 {
            self.glyf.as_ref()
        } else {
            self.more_fonts[self.current_font - 1].glyf.as_ref()
        }
    }

    fn current_colr(&self) -> Option<&colr::COLR> {
        if self.current_font == 0 {
            self.colr.as_ref()
        } else {
            self.more_fonts[self.current_font - 1].colr.as_ref()
        }
    }

    fn current_cpal(&self) -> Option<&cpal::CPAL> {
        if self.current_font == 0 {
            self.cpal.as_ref()
        } else {
            self.more_fonts[self.current_font - 1].cpal.as_ref()
        }
    }

    fn current_sbix(&self) -> Option<&sbix::SBIX> {
        if self.current_font == 0 {
            self.sbix.as_ref()
        } else {
            self.more_fonts[self.current_font - 1].sbix.as_ref()
        }
    }

    fn current_svg_table(&self) -> Option<&svg::SVG> {
        if self.current_font == 0 {
            self.svg.as_ref()
        } else {
            self.more_fonts[self.current_font - 1].svg.as_ref()
        }
    }

    #[cfg(feature = "layout")]
    fn current_gpos(&self) -> Option<&gpos::GPOS> {
        if self.current_font == 0 {
            self.gpos.as_ref()
        } else {
            self.more_fonts[self.current_font - 1].gpos.as_ref()
        }
    }

    #[cfg(feature = "layout")]
    fn current_gsub(&self) -> Option<&gsub::GSUB> {
        if self.current_font == 0 {
            self.gsub.as_ref()
        } else {
            self.more_fonts[self.current_font - 1].gsub.as_ref()
        }
    }

    #[cfg(feature = "cff")]
    fn current_cff(&self) -> Option<&cff::CFF> {
        if self.current_font == 0 {
            self.cff.as_ref()
        } else {
            self.more_fonts[self.current_font - 1].cff.as_ref()
        }
    }

    fn get_glyph_from_id_axis(&self, glyph_id: usize, is_vert: bool) -> GriphData {
        let layout = self.get_layout(glyph_id, is_vert);

        #[cfg(feature = "cff")]
        if let Some(cff) = self.current_cff() {
            let string = cff.to_code(glyph_id, &layout);
            let open_type_glyf = Some(OpenTypeGlyph {
                layout,
                glyph: FontData::CFF(string.as_bytes().to_vec()),
            });

            return GriphData {
                glyph_id,
                open_type_glyf,
            };
        }

        let glyf = self.current_glyf().unwrap();
        let glyph = glyf.get_glyph(glyph_id).unwrap();
        let open_type_glyph = OpenTypeGlyph {
            layout,
            glyph: FontData::Glyph(glyph.clone()),
        };

        GriphData {
            glyph_id,
            open_type_glyf: Some(open_type_glyph),
        }
    }

    fn resolve_glyph_id_with_uvs(&self, ch: char, vs: char, is_vert: bool) -> Result<usize, Error> {
        let glyph_id = self
            .current_cmap()?
            .get_glyph_position_from_uvs(ch as u32, vs as u32) as usize;

        #[cfg(feature = "layout")]
        {
            if is_vert {
                if let Some(gsub) = self.current_gsub() {
                    return Ok(gsub
                        .lookup_vertical(glyph_id as u16)
                        .unwrap_or(glyph_id as u16) as usize);
                }
            }
        }

        #[cfg(not(feature = "layout"))]
        let _ = is_vert;

        Ok(glyph_id)
    }

    fn is_variation_selector(ch: char) -> bool {
        (0xfe00..=0xfe0f).contains(&(ch as u32)) || (0xE0100..=0xE01EF).contains(&(ch as u32))
    }

    fn is_emoji_modifier(ch: char) -> bool {
        (0x1F3FB..=0x1F3FF).contains(&(ch as u32))
    }

    fn is_zero_width_joiner(ch: char) -> bool {
        ch == '\u{200D}'
    }

    fn is_keycap_mark(ch: char) -> bool {
        ch == '\u{20E3}'
    }

    fn is_regional_indicator(ch: char) -> bool {
        (0x1F1E6..=0x1F1FF).contains(&(ch as u32))
    }

    fn is_tag_character(ch: char) -> bool {
        (0xE0020..=0xE007E).contains(&(ch as u32)) || ch == '\u{E007F}'
    }

    fn is_default_emoji_scalar(ch: char) -> bool {
        matches!(ch as u32, 0x1F000..=0x1FAFF | 0x2600..=0x27BF)
    }

    fn text_prefers_color_glyph(text: &str) -> bool {
        let mut saw_emoji_scalar = false;
        for ch in text.chars() {
            if Self::is_variation_selector(ch)
                || Self::is_emoji_modifier(ch)
                || Self::is_zero_width_joiner(ch)
                || Self::is_keycap_mark(ch)
                || Self::is_regional_indicator(ch)
                || Self::is_tag_character(ch)
            {
                return true;
            }
            if Self::is_default_emoji_scalar(ch) {
                saw_emoji_scalar = true;
            }
        }
        saw_emoji_scalar
    }

    fn extend_cluster_suffix(chars: &[char], index: &mut usize, text: &mut String) {
        while *index < chars.len() {
            let ch = chars[*index];
            if Self::is_variation_selector(ch)
                || Self::is_emoji_modifier(ch)
                || Self::is_keycap_mark(ch)
                || Self::is_tag_character(ch)
            {
                text.push(ch);
                *index += 1;
            } else {
                break;
            }
        }
    }

    fn cluster_glyph_scalars(text: &str) -> Vec<(char, char)> {
        let chars: Vec<char> = text.chars().collect();
        let mut glyphs = Vec::new();
        let mut index = 0usize;

        while index < chars.len() {
            let ch = chars[index];
            if Self::is_variation_selector(ch) {
                index += 1;
                continue;
            }

            let mut variation_selector = '\0';
            if index + 1 < chars.len() && Self::is_variation_selector(chars[index + 1]) {
                variation_selector = chars[index + 1];
                index += 1;
            }

            glyphs.push((ch, variation_selector));
            index += 1;
        }

        glyphs
    }

    fn parse_text_units(text: &str) -> Vec<ParsedTextUnit> {
        let chars: Vec<char> = text.chars().collect();
        let mut units = Vec::new();
        let mut index = 0;

        while index < chars.len() {
            let ch = chars[index];
            match ch {
                '\r' => {
                    index += 1;
                }
                '\n' => {
                    units.push(ParsedTextUnit::Newline);
                    index += 1;
                }
                '\t' => {
                    units.push(ParsedTextUnit::Tab);
                    index += 1;
                }
                _ if Self::is_variation_selector(ch) => {
                    index += 1;
                }
                _ => {
                    let mut text = String::new();
                    text.push(ch);
                    let mut variation_selector = '\0';
                    if index + 1 < chars.len() && Self::is_variation_selector(chars[index + 1]) {
                        variation_selector = chars[index + 1];
                        text.push(variation_selector);
                        index += 1;
                    }

                    index += 1;
                    Self::extend_cluster_suffix(&chars, &mut index, &mut text);

                    if Self::is_regional_indicator(ch)
                        && index < chars.len()
                        && Self::is_regional_indicator(chars[index])
                    {
                        text.push(chars[index]);
                        index += 1;
                        Self::extend_cluster_suffix(&chars, &mut index, &mut text);
                    }

                    while index < chars.len() && Self::is_zero_width_joiner(chars[index]) {
                        text.push(chars[index]);
                        index += 1;
                        if index >= chars.len() {
                            break;
                        }
                        text.push(chars[index]);
                        index += 1;
                        Self::extend_cluster_suffix(&chars, &mut index, &mut text);
                    }

                    units.push(ParsedTextUnit::Glyph {
                        text,
                        ch,
                        variation_selector,
                    });
                }
            }
        }

        units
    }

    pub(crate) fn parse_text_units_for_fallback(text: &str) -> Vec<ParsedTextUnit> {
        Self::parse_text_units(text)
    }

    fn flush_shaped_glyphs(
        &self,
        output: &mut Vec<ResolvedTextUnit>,
        glyphs: &mut Vec<ResolvedGlyph>,
        locale: Option<&str>,
        is_right_to_left: bool,
        font_variant: crate::commands::FontVariant,
    ) {
        #[cfg(not(feature = "layout"))]
        let _ = (locale, is_right_to_left, font_variant);

        if glyphs.is_empty() {
            return;
        }

        #[cfg(feature = "layout")]
        if let Some(gsub) = self.current_gsub() {
            const MAX_LIGATURE_COMPONENTS: usize = 8;

            let mut ccmp_glyphs = glyphs
                .iter()
                .enumerate()
                .map(|(source_index, glyph)| (glyph.glyph_id, source_index))
                .collect::<Vec<_>>();
            gsub.apply_ccmp_sequence(&mut ccmp_glyphs);
            gsub.apply_variant_sequence(&mut ccmp_glyphs, locale, font_variant);
            if is_right_to_left {
                gsub.apply_joining_sequence(&mut ccmp_glyphs, locale);
                gsub.apply_rtl_contextual_sequence(&mut ccmp_glyphs, locale);
            }
            let expanded_glyphs = ccmp_glyphs
                .into_iter()
                .map(|(glyph_id, source_index)| ResolvedGlyph {
                    ch: glyphs[source_index].ch,
                    glyph_id,
                    prefer_color: glyphs[source_index].prefer_color,
                })
                .collect::<Vec<_>>();

            let glyph_ids: Vec<usize> =
                expanded_glyphs.iter().map(|glyph| glyph.glyph_id).collect();
            let mut index = 0;
            while index < expanded_glyphs.len() {
                let max_len = (expanded_glyphs.len() - index).min(MAX_LIGATURE_COMPONENTS);
                let mut matched = None;
                for len in (2..=max_len).rev() {
                    if is_right_to_left {
                        if let Some(glyph_id) =
                            gsub.lookup_rlig_sequence(&glyph_ids[index..index + len], locale)
                        {
                            matched = Some((glyph_id, len));
                            break;
                        }
                    }
                    if let Some(glyph_id) =
                        gsub.lookup_liga_sequence(&glyph_ids[index..index + len])
                    {
                        matched = Some((glyph_id, len));
                        break;
                    }
                }

                if let Some((glyph_id, len)) = matched {
                    output.push(ResolvedTextUnit::Glyph(ResolvedGlyph {
                        ch: expanded_glyphs[index].ch,
                        glyph_id,
                        prefer_color: expanded_glyphs[index].prefer_color,
                    }));
                    index += len;
                } else {
                    output.push(ResolvedTextUnit::Glyph(expanded_glyphs[index]));
                    index += 1;
                }
            }
            glyphs.clear();
            return;
        }

        output.extend(glyphs.iter().copied().map(ResolvedTextUnit::Glyph));
        glyphs.clear();
    }

    fn shape_text_units(
        &self,
        text: &str,
        is_vert: bool,
        is_right_to_left: bool,
        locale: Option<&str>,
        font_variant: crate::commands::FontVariant,
    ) -> Result<Vec<ResolvedTextUnit>, Error> {
        #[cfg(not(feature = "layout"))]
        let _ = (locale, is_right_to_left, font_variant);

        let mut output = Vec::new();
        let mut pending_glyphs = Vec::new();

        for unit in Self::parse_text_units(text) {
            match unit {
                ParsedTextUnit::Newline => {
                    self.flush_shaped_glyphs(
                        &mut output,
                        &mut pending_glyphs,
                        locale,
                        is_right_to_left,
                        font_variant,
                    );
                    output.push(ResolvedTextUnit::Newline);
                }
                ParsedTextUnit::Tab => {
                    self.flush_shaped_glyphs(
                        &mut output,
                        &mut pending_glyphs,
                        locale,
                        is_right_to_left,
                        font_variant,
                    );
                    output.push(ResolvedTextUnit::Tab);
                }
                ParsedTextUnit::Glyph { text, .. } => {
                    let prefer_color = Self::text_prefers_color_glyph(&text);
                    for (ch, variation_selector) in Self::cluster_glyph_scalars(&text) {
                        let glyph_id =
                            self.resolve_glyph_id_with_uvs(ch, variation_selector, is_vert)?;
                        #[cfg(feature = "layout")]
                        let glyph_id = if let Some(locale) = locale {
                            if let Some(gsub) = self.current_gsub() {
                                gsub.lookup_locale(glyph_id, locale)
                            } else {
                                glyph_id
                            }
                        } else {
                            glyph_id
                        };
                        pending_glyphs.push(ResolvedGlyph {
                            ch,
                            glyph_id,
                            prefer_color,
                        });
                    }
                }
            }
        }

        self.flush_shaped_glyphs(
            &mut output,
            &mut pending_glyphs,
            locale,
            is_right_to_left,
            font_variant,
        );
        Ok(output)
    }

    pub(crate) fn supports_text_unit(
        &self,
        unit: &ParsedTextUnit,
        is_vert: bool,
        locale: Option<&str>,
    ) -> bool {
        #[cfg(not(feature = "layout"))]
        let _ = locale;

        match unit {
            ParsedTextUnit::Newline | ParsedTextUnit::Tab => true,
            ParsedTextUnit::Glyph { text, .. } => {
                let Ok(shaped_units) = self.shape_text_units(
                    text,
                    is_vert,
                    false,
                    locale,
                    crate::commands::FontVariant::Normal,
                ) else {
                    return false;
                };

                if !shaped_units
                    .iter()
                    .any(|unit| matches!(unit, ResolvedTextUnit::Glyph(_)))
                {
                    return false;
                }

                shaped_units.into_iter().all(|unit| match unit {
                    ResolvedTextUnit::Glyph(glyph) => {
                        if glyph.glyph_id == 0 {
                            return false;
                        }

                        #[cfg(feature = "cff")]
                        if self.current_cff().is_some() {
                            return true;
                        }

                        self.current_glyf()
                            .and_then(|glyf| glyf.get_glyph(glyph.glyph_id))
                            .is_some()
                            || self.current_sbix()
                                .and_then(|sbix| sbix.get_raster_glyph(glyph.glyph_id as u32, 16.0, "px"))
                                .is_some()
                            || self
                                .current_svg_table()
                                .map(|svg| svg.has_glyph(glyph.glyph_id as u32))
                                .unwrap_or(false)
                    }
                    _ => true,
                })
            }
        }
    }

    fn push_svg_html_unit(
        &self,
        svgs: &mut Vec<String>,
        unit: ParsedTextUnit,
        fontsize: f64,
        fontunit: &str,
        is_vert: bool,
    ) -> Result<(), Error> {
        match unit {
            ParsedTextUnit::Newline => {
                svgs.push("<br>".to_string());
            }
            ParsedTextUnit::Tab => {
                svgs.push(
                    "<span style=\"width: 4em; display: inline-block;\"></span>\n".to_string(),
                );
            }
            ParsedTextUnit::Glyph {
                text,
                ch,
                variation_selector,
            } => {
                let svg = if text.chars().count() > 2
                    || text.contains('\u{200D}')
                    || text.chars().filter(|ch| Self::is_regional_indicator(*ch)).count() > 1
                {
                    self.text2svg(&text, fontsize, fontunit)?
                } else {
                    self.get_svg_with_uvs_axis(ch, variation_selector, fontsize, fontunit, is_vert)?
                };
                svgs.push(svg);
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn debug_shape_glyph_ids(
        &self,
        text: &str,
        locale: Option<&str>,
    ) -> Result<Vec<usize>, Error> {
        let mut glyph_ids = Vec::new();
        for unit in self.shape_text_units(
            text,
            false,
            false,
            locale,
            crate::commands::FontVariant::Normal,
        )? {
            if let ResolvedTextUnit::Glyph(glyph) = unit {
                glyph_ids.push(glyph.glyph_id);
            }
        }
        Ok(glyph_ids)
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn debug_shape_glyph_ids_with_direction(
        &self,
        text: &str,
        locale: Option<&str>,
        is_right_to_left: bool,
    ) -> Result<Vec<usize>, Error> {
        let mut glyph_ids = Vec::new();
        for unit in self.shape_text_units(
            text,
            false,
            is_right_to_left,
            locale,
            crate::commands::FontVariant::Normal,
        )? {
            if let ResolvedTextUnit::Glyph(glyph) = unit {
                glyph_ids.push(glyph.glyph_id);
            }
        }
        Ok(glyph_ids)
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn debug_shape_glyph_ids_with_variant(
        &self,
        text: &str,
        locale: Option<&str>,
        font_variant: crate::commands::FontVariant,
    ) -> Result<Vec<usize>, Error> {
        let mut glyph_ids = Vec::new();
        for unit in self.shape_text_units(text, false, false, locale, font_variant)? {
            if let ResolvedTextUnit::Glyph(glyph) = unit {
                glyph_ids.push(glyph.glyph_id);
            }
        }
        Ok(glyph_ids)
    }

    fn default_line_height(&self) -> Result<f64, Error> {
        let hhea = self.current_hhea()?;
        Ok((hhea.get_accender() - hhea.get_descender() + hhea.get_line_gap()) as f64)
    }

    #[allow(dead_code)]
    fn glyph_unit_at(units: &[ResolvedTextUnit], index: usize) -> Option<ResolvedGlyph> {
        match units.get(index) {
            Some(ResolvedTextUnit::Glyph(glyph)) => Some(*glyph),
            _ => None,
        }
    }

    fn resolved_glyph_can_use_outline(
        &self,
        open_type_glyph: &OpenTypeGlyph,
        glyph_id: usize,
    ) -> bool {
        if self
            .current_colr()
            .map(|colr| !colr.get_layer_record(glyph_id as u16).is_empty())
            .unwrap_or(false)
        {
            return true;
        }

        #[cfg(feature = "cff")]
        if self.current_cff().is_some() {
            return true;
        }

        matches!(open_type_glyph.glyph, FontData::Glyph(_))
    }

    fn pair_adjustment_for_index(
        &self,
        units: &[ResolvedTextUnit],
        index: usize,
        locale: Option<&str>,
        is_vertical: bool,
        scale_x: f32,
        scale_y: f32,
    ) -> GlyphPositionAdjustment {
        #[cfg(not(feature = "layout"))]
        {
            let _ = (units, index, locale, is_vertical, scale_x, scale_y);
            GlyphPositionAdjustment::default()
        }

        #[cfg(feature = "layout")]
        {
            let Some(gpos) = self.current_gpos() else {
                return GlyphPositionAdjustment::default();
            };
            let Some(current) = Self::glyph_unit_at(units, index) else {
                return GlyphPositionAdjustment::default();
            };

            let mut adjustment = GlyphPositionAdjustment::default();

            if index > 0 {
                if let Some(previous) = Self::glyph_unit_at(units, index - 1) {
                    if let Some(pair) = gpos.lookup_pair_adjustment(
                        previous.glyph_id as u16,
                        current.glyph_id as u16,
                        is_vertical,
                        locale,
                    ) {
                        adjustment.placement_x += pair.second.x_placement as f32 * scale_x;
                        adjustment.placement_y += pair.second.y_placement as f32 * scale_y;
                        adjustment.advance_x += pair.second.x_advance as f32 * scale_x;
                        adjustment.advance_y += pair.second.y_advance as f32 * scale_y;
                    }
                }
            }

            if let Some(next) = Self::glyph_unit_at(units, index + 1) {
                if let Some(pair) = gpos.lookup_pair_adjustment(
                    current.glyph_id as u16,
                    next.glyph_id as u16,
                    is_vertical,
                    locale,
                ) {
                    adjustment.placement_x += pair.first.x_placement as f32 * scale_x;
                    adjustment.placement_y += pair.first.y_placement as f32 * scale_y;
                    adjustment.advance_x += pair.first.x_advance as f32 * scale_x;
                    adjustment.advance_y += pair.first.y_advance as f32 * scale_y;
                }
            }

            adjustment
        }
    }

    pub fn text2glyph_run(
        &self,
        text: &str,
        options: &crate::commands::FontOptions<'_>,
    ) -> Result<GlyphRun, Error> {
        let _ = self.current_head()?;

        if !options.font_size.is_finite() || options.font_size <= 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "font_size must be a positive finite value",
            ));
        }

        let default_line_height = self.default_line_height()? as f32;
        let scale_y = options.font_size / default_line_height.max(1.0);
        let scale_x = scale_y * options.font_stretch.0.max(0.0);
        let line_height = options.line_height.unwrap_or(options.font_size);
        let is_vertical = options.text_direction.is_vertical();
        let is_right_to_left = options.text_direction.is_right_to_left();
        if !line_height.is_finite() || line_height <= 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "line_height must be a positive finite value",
            ));
        }

        let mut glyphs = Vec::new();
        let mut cursor_x = 0.0f32;
        let mut cursor_y = 0.0f32;
        let tab_advance = line_height;
        let shaped_units = self.shape_text_units(
            text,
            is_vertical,
            is_right_to_left,
            options.locale,
            options.font_variant,
        )?;

        for (index, unit) in shaped_units.iter().enumerate() {
            match *unit {
                ResolvedTextUnit::Newline => {
                    if is_vertical {
                        cursor_x -= line_height;
                        cursor_y = 0.0;
                    } else {
                        cursor_x = 0.0;
                        cursor_y += line_height;
                    }
                }
                ResolvedTextUnit::Tab => {
                    if is_vertical {
                        cursor_y += tab_advance * 4.0;
                    } else if is_right_to_left {
                        cursor_x -= tab_advance * 4.0;
                    } else {
                        cursor_x += tab_advance * 4.0;
                    }
                }
                ResolvedTextUnit::Glyph(resolved) => {
                    let glyph_data = self.get_glyph_from_id_axis(resolved.glyph_id, is_vertical);
                    let open_type_glyph = glyph_data
                        .open_type_glyf
                        .as_ref()
                        .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "glyph is none"))?;
                    let glyph_id = glyph_data.glyph_id;
                    let can_use_outline =
                        self.resolved_glyph_can_use_outline(open_type_glyph, glyph_id);

                    let layers = if let Some(sbix) = self.current_sbix() {
                        if let Some(bitmap) =
                            sbix.get_raster_glyph(glyph_id as u32, options.font_size, "px")
                        {
                            if resolved.prefer_color || !can_use_outline {
                                let mut raster = RasterGlyphLayer::from_encoded(bitmap.glyph_data);
                                raster.offset_x =
                                    bitmap.offset_x * options.font_stretch.0.max(0.0);
                                raster.offset_y = bitmap.offset_y;
                                raster.width = bitmap.width;
                                raster.height = bitmap.height;
                                vec![GlyphLayer::Raster(raster)]
                            } else {
                                self.build_outline_layers(
                                    glyph_id,
                                    open_type_glyph,
                                    scale_x,
                                    scale_y,
                                    resolved.ch,
                                )?
                            }
                        } else {
                            self.build_outline_layers(
                                glyph_id,
                                open_type_glyph,
                                scale_x,
                                scale_y,
                                resolved.ch,
                            )?
                        }
                    } else {
                        self.build_outline_layers(
                            glyph_id,
                            open_type_glyph,
                            scale_x,
                            scale_y,
                            resolved.ch,
                        )?
                    };

                    let mut metrics =
                        glyph_metrics_from_layout(&open_type_glyph.layout, scale_x, scale_y);
                    let adjustment = self.pair_adjustment_for_index(
                        &shaped_units,
                        index,
                        options.locale,
                        is_vertical,
                        scale_x,
                        scale_y,
                    );
                    metrics.advance_x += adjustment.advance_x;
                    metrics.advance_y += adjustment.advance_y;
                    metrics.bounds = glyph_layers_bounds(&layers);

                    let origin_x = if is_right_to_left && !is_vertical {
                        cursor_x - metrics.advance_x + adjustment.placement_x
                    } else {
                        cursor_x + adjustment.placement_x
                    };
                    let origin_y = cursor_y + adjustment.placement_y;
                    let glyph = Glyph {
                        font: Some(font_metrics_from_layout(&open_type_glyph.layout, scale_y)),
                        metrics,
                        layers,
                    };
                    glyphs.push(PositionedGlyph::new(glyph, origin_x, origin_y));
                    if is_right_to_left && !is_vertical {
                        cursor_x -= metrics.advance_x;
                    } else {
                        cursor_x += metrics.advance_x;
                    }
                    cursor_y += metrics.advance_y;
                }
            }
        }

        Ok(GlyphRun::new(glyphs))
    }

    fn build_outline_layers(
        &self,
        glyph_id: usize,
        open_type_glyph: &OpenTypeGlyph,
        scale_x: f32,
        scale_y: f32,
        ch: char,
    ) -> Result<Vec<GlyphLayer>, Error> {
        if self
            .current_svg_table()
            .map(|svg| svg.has_glyph(glyph_id as u32))
            .unwrap_or(false)
        {
            return Err(Error::new(
                ErrorKind::Unsupported,
                format!("SVG glyph layers are not supported yet for {:?}", ch),
            ));
        }

        let color_layers =
            self.build_colr_layers(glyph_id, &open_type_glyph.layout, scale_x, scale_y);
        if !color_layers.is_empty() {
            return Ok(color_layers);
        }

        #[cfg(feature = "cff")]
        if let Some(cff) = self.current_cff() {
            let commands = cff.to_path_commands(glyph_id, 1.0)?;
            let commands = transform_cff_commands(&commands, scale_x, scale_y);
            return Ok(vec![GlyphLayer::Path(PathGlyphLayer::new(
                commands,
                GlyphPaint::CurrentColor,
            ))]);
        }

        match &open_type_glyph.glyph {
            FontData::Glyph(_) => {
                let glyf = self
                    .current_glyf()
                    .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "glyf is none"))?;
                let commands = glyf.to_path_commands(glyph_id, &open_type_glyph.layout, 0.0, 0.0);
                let commands =
                    transform_glyf_commands(&commands, &open_type_glyph.layout, scale_x, scale_y);
                Ok(vec![GlyphLayer::Path(PathGlyphLayer::new(
                    commands,
                    GlyphPaint::CurrentColor,
                ))])
            }
            FontData::Bitmap(_, _) => Err(Error::new(
                ErrorKind::Unsupported,
                "bitmap glyphs are only supported through sbix raster layers",
            )),
            FontData::SVG(_) => Err(Error::new(
                ErrorKind::Unsupported,
                "SVG glyph layers are not supported yet",
            )),
            _ => Err(Error::new(
                ErrorKind::Unsupported,
                "glyph outlines are not available for this font",
            )),
        }
    }

    fn build_colr_layers(
        &self,
        glyph_id: usize,
        layout: &FontLayout,
        scale_x: f32,
        scale_y: f32,
    ) -> Vec<GlyphLayer> {
        let (Some(colr), Some(cpal), Some(glyf)) = (
            self.current_colr(),
            self.current_cpal(),
            self.current_glyf(),
        ) else {
            return Vec::new();
        };

        let mut layers = Vec::new();
        for layer in colr.get_layer_record(glyph_id as u16) {
            if glyf.get_glyph(layer.glyph_id as usize).is_none() {
                continue;
            }
            let commands = glyf.to_path_commands(layer.glyph_id as usize, layout, 0.0, 0.0);
            let commands = transform_glyf_commands(&commands, layout, scale_x, scale_y);
            let color = cpal.get_pallet(layer.palette_index as usize);
            let argb = ((color.alpha as u32) << 24)
                | ((color.red as u32) << 16)
                | ((color.green as u32) << 8)
                | color.blue as u32;
            layers.push(GlyphLayer::Path(PathGlyphLayer::new(
                commands,
                GlyphPaint::Solid(argb),
            )));
        }

        layers
    }

    fn legacy_colr_commands(
        &self,
        glyph_id: usize,
        layout: &FontLayout,
        origin_x: f64,
        origin_y: f64,
    ) -> Vec<PathCommand> {
        let (Some(colr), Some(glyf)) = (self.current_colr(), self.current_glyf()) else {
            return Vec::new();
        };

        let mut commands = Vec::new();
        for layer in colr.get_layer_record(glyph_id as u16) {
            if glyf.get_glyph(layer.glyph_id as usize).is_none() {
                continue;
            }
            commands.extend(glyf.to_path_commands(
                layer.glyph_id as usize,
                layout,
                origin_x,
                origin_y,
            ));
        }
        commands
    }

    pub(crate) fn text2commands(&self, text: &str) -> Result<Vec<GlyphCommands>, Error> {
        let mut result = Vec::new();
        let mut cursor_x = 0.0;
        let mut line_index = 0usize;
        let line_height = self.default_line_height()?;
        let tab_advance = line_height;
        let shaped_units = self.shape_text_units(
            text,
            false,
            false,
            None,
            crate::commands::FontVariant::Normal,
        )?;

        for (index, unit) in shaped_units.iter().enumerate() {
            match *unit {
                ResolvedTextUnit::Newline => {
                    cursor_x = 0.0;
                    line_index += 1;
                }
                ResolvedTextUnit::Tab => {
                    cursor_x += tab_advance * 4.0;
                }
                ResolvedTextUnit::Glyph(resolved) => {
                    let glyph_data = self.get_glyph_from_id_axis(resolved.glyph_id, false);
                    let open_type_glyph = glyph_data
                        .open_type_glyf
                        .as_ref()
                        .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "glyph is none"))?;
                    let adjustment =
                        self.pair_adjustment_for_index(&shaped_units, index, None, false, 1.0, 1.0);
                    let origin_y =
                        -(line_index as f64 * line_height) + adjustment.placement_y as f64;
                    let advance_width = match &open_type_glyph.layout {
                        FontLayout::Horizontal(layout) => layout.advance_width as f64,
                        FontLayout::Vertical(layout) => layout.advance_height as f64,
                        FontLayout::Unknown => 0.0,
                    } + adjustment.advance_x as f64;
                    let origin_x = cursor_x + adjustment.placement_x as f64;
                    let can_use_outline =
                        self.resolved_glyph_can_use_outline(open_type_glyph, glyph_data.glyph_id);

                    if let Some(sbix) = self.current_sbix() {
                        if let Some(bitmap) = sbix.get_raster_glyph(
                            glyph_data.glyph_id as u32,
                            line_height as f32,
                            "px",
                        ) {
                            if resolved.prefer_color || !can_use_outline {
                                let format =
                                    if bitmap.graphic_type == u32::from_be_bytes(*b"png ") {
                                        BitmapGlyphFormat::Png
                                    } else if bitmap.graphic_type == u32::from_be_bytes(*b"jpg ") {
                                        BitmapGlyphFormat::Jpeg
                                    } else {
                                        return Err(Error::new(
                                            std::io::ErrorKind::Unsupported,
                                            "unsupported sbix image format",
                                        ));
                                    };
                                let sniffed_dimensions =
                                    sniff_encoded_image_dimensions(&bitmap.glyph_data);
                                result.push(GlyphCommands {
                                    ch: resolved.ch,
                                    glyph_id: glyph_data.glyph_id,
                                    origin_x,
                                    origin_y,
                                    advance_width,
                                    commands: Vec::new(),
                                    bitmap: Some(BitmapGlyphCommands {
                                        offset_x: bitmap.offset_x as f64,
                                        offset_y: bitmap.offset_y as f64,
                                        width: bitmap
                                            .width
                                            .map(|width| width as f64)
                                            .or_else(|| {
                                                sniffed_dimensions
                                                    .map(|(_, width, _)| width as f64)
                                            })
                                            .unwrap_or(line_height),
                                        height: bitmap
                                            .height
                                            .map(|height| height as f64)
                                            .or_else(|| {
                                                sniffed_dimensions
                                                    .map(|(_, _, height)| height as f64)
                                            })
                                            .unwrap_or(line_height),
                                        format,
                                        data: bitmap.glyph_data,
                                    }),
                                });
                                cursor_x += advance_width;
                                continue;
                            }
                        }
                    }

                    match &open_type_glyph.glyph {
                        FontData::Glyph(_) => {
                            let mut commands = self.legacy_colr_commands(
                                glyph_data.glyph_id,
                                &open_type_glyph.layout,
                                origin_x,
                                origin_y,
                            );
                            if commands.is_empty() {
                                let glyf = self.current_glyf().ok_or_else(|| {
                                    Error::new(std::io::ErrorKind::Other, "glyf is none")
                                })?;
                                commands = glyf.to_path_commands(
                                    glyph_data.glyph_id,
                                    &open_type_glyph.layout,
                                    origin_x,
                                    origin_y,
                                );
                            }
                            result.push(GlyphCommands {
                                ch: resolved.ch,
                                glyph_id: glyph_data.glyph_id,
                                origin_x,
                                origin_y,
                                advance_width,
                                commands,
                                bitmap: None,
                            });
                            cursor_x += advance_width;
                        }
                        _ => {
                            return Err(Error::new(
                                std::io::ErrorKind::Other,
                                "text2commands supports glyf outlines and sbix bitmap glyphs only",
                            ));
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    #[cfg(test)]
    pub(crate) fn text2command(&self, text: &str) -> Result<Vec<GlyphCommands>, Error> {
        self.text2commands(text)
    }

    pub fn measure(&self, text: &str) -> Result<f64, Error> {
        self.measure_with_options(text, &crate::commands::FontOptions::from_font(self))
    }

    pub fn measure_with_options(
        &self,
        text: &str,
        options: &crate::commands::FontOptions<'_>,
    ) -> Result<f64, Error> {
        let mut cursor_x = 0.0;
        let mut cursor_y = 0.0;
        let mut max_line_width: f64 = 0.0;
        let line_height = self.default_line_height()?;
        let tab_advance = line_height;
        let is_vertical = options.text_direction.is_vertical();
        let is_right_to_left = options.text_direction.is_right_to_left();
        let shaped_units = self.shape_text_units(
            text,
            is_vertical,
            is_right_to_left,
            options.locale,
            options.font_variant,
        )?;

        for (index, unit) in shaped_units.iter().enumerate() {
            match *unit {
                ResolvedTextUnit::Newline => {
                    max_line_width = if is_vertical {
                        max_line_width.max(cursor_y)
                    } else if is_right_to_left {
                        max_line_width.max(-cursor_x)
                    } else {
                        max_line_width.max(cursor_x)
                    };
                    if is_vertical {
                        cursor_x -= line_height;
                        cursor_y = 0.0;
                    } else {
                        cursor_x = 0.0;
                    }
                }
                ResolvedTextUnit::Tab => {
                    if is_vertical {
                        cursor_y += tab_advance * 4.0;
                    } else if is_right_to_left {
                        cursor_x -= tab_advance * 4.0;
                    } else {
                        cursor_x += tab_advance * 4.0;
                    }
                }
                ResolvedTextUnit::Glyph(resolved) => {
                    let glyph_data = self.get_glyph_from_id_axis(resolved.glyph_id, is_vertical);
                    let open_type_glyph = glyph_data
                        .open_type_glyf
                        .as_ref()
                        .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "glyph is none"))?;

                    let adjustment = self.pair_adjustment_for_index(
                        &shaped_units,
                        index,
                        options.locale,
                        is_vertical,
                        1.0,
                        1.0,
                    );
                    let (advance_x, advance_y) = match &open_type_glyph.layout {
                        FontLayout::Horizontal(layout) => (layout.advance_width as f64, 0.0),
                        FontLayout::Vertical(layout) => (0.0, layout.advance_height as f64),
                        FontLayout::Unknown => (0.0, 0.0),
                    };
                    if is_right_to_left && !is_vertical {
                        cursor_x -= advance_x + adjustment.advance_x as f64;
                    } else {
                        cursor_x += advance_x + adjustment.advance_x as f64;
                    }
                    cursor_y += advance_y + adjustment.advance_y as f64;
                }
            }
        }

        Ok(if is_vertical {
            max_line_width.max(cursor_y)
        } else if is_right_to_left {
            max_line_width.max(-cursor_x)
        } else {
            max_line_width.max(cursor_x)
        })
    }

    pub(crate) fn text2svg(&self, text: &str, fontsize: f64, fontunit: &str) -> Result<String, Error> {
        let glyphs = self.text2commands(text)?;
        let line_height = self.default_line_height()?;
        let mut svg_elements = Vec::new();
        let mut min_x = 0.0;
        let mut min_y = 0.0;
        let mut max_x = 0.0;
        let mut max_y = 0.0;
        let mut has_point = false;

        for glyph in glyphs.iter() {
            let d = path_commands_to_svg_path(&glyph.commands);
            if !d.is_empty() {
                let (glyph_min_x, glyph_min_y, glyph_max_x, glyph_max_y) =
                    path_command_bounds(&glyph.commands);
                if !has_point {
                    min_x = glyph_min_x;
                    min_y = glyph_min_y;
                    max_x = glyph_max_x;
                    max_y = glyph_max_y;
                    has_point = true;
                } else {
                    min_x = min_x.min(glyph_min_x);
                    min_y = min_y.min(glyph_min_y);
                    max_x = max_x.max(glyph_max_x);
                    max_y = max_y.max(glyph_max_y);
                }
                svg_elements.push(format!("<path d=\"{}\" fill=\"currentColor\"/>", d));
            }

            if let Some(bitmap) = glyph.bitmap.as_ref() {
                let glyph_min_x = glyph.origin_x + bitmap.offset_x;
                let glyph_min_y = glyph.origin_y + bitmap.offset_y;
                let glyph_max_x = glyph_min_x + bitmap.width;
                let glyph_max_y = glyph_min_y + bitmap.height;
                if !has_point {
                    min_x = glyph_min_x;
                    min_y = glyph_min_y;
                    max_x = glyph_max_x;
                    max_y = glyph_max_y;
                    has_point = true;
                } else {
                    min_x = min_x.min(glyph_min_x);
                    min_y = min_y.min(glyph_min_y);
                    max_x = max_x.max(glyph_max_x);
                    max_y = max_y.max(glyph_max_y);
                }
                svg_elements.push(bitmap_glyph_to_svg_image(glyph, bitmap));
            }
        }

        if !has_point {
            let size = format!("0{}", fontunit);
            return Ok(format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 0 0\"></svg>",
                size, size
            ));
        }

        const SVG_EXPORT_PADDING: f64 = 2.0;
        min_x -= SVG_EXPORT_PADDING;
        min_y -= SVG_EXPORT_PADDING;
        let view_width = (max_x - min_x + SVG_EXPORT_PADDING).max(1.0);
        let view_height = (max_y - min_y + SVG_EXPORT_PADDING).max(1.0);
        let scale = fontsize / line_height.max(1.0);
        let width = (view_width * scale).ceil();
        let height = (view_height * scale).ceil();

        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}{}\" height=\"{}{}\" viewBox=\"{} {} {} {}\" overflow=\"visible\">",
            width, fontunit, height, fontunit, min_x, min_y, view_width, view_height
        );
        for element in svg_elements {
            svg += &element;
        }
        svg += "</svg>";
        Ok(svg)
    }

    pub fn get_name(&self, name_id: NameID, locale: &String) -> Result<String, Error> {
        let name_table = if self.current_font == 0 {
            if self.name_table.is_none() {
                return Err(Error::new(
                    std::io::ErrorKind::Other,
                    "name table is none".to_string(),
                ));
            }
            self.name_table.as_ref().unwrap()
        } else {
            if self.more_fonts[self.current_font - 1].name_table.is_none() {
                return Err(Error::new(
                    std::io::ErrorKind::Other,
                    "name table is none".to_string(),
                ));
            }
            self.more_fonts[self.current_font - 1]
                .name_table
                .as_ref()
                .unwrap()
        };
        let platform_id = PlatformID::Windows;
        name_table.get_name(name_id, locale, platform_id)
    }

    pub(crate) fn face_family_name(&self) -> String {
        let locale = "en-US".to_string();
        self.get_name(NameID::TypographicFamilyName, &locale)
            .ok()
            .filter(|name| !name.trim().is_empty())
            .or_else(|| {
                self.get_name(NameID::FontFamilyName, &locale)
                    .ok()
                    .filter(|name| !name.trim().is_empty())
            })
            .unwrap_or_else(|| "Unknown Family".to_string())
    }

    pub(crate) fn face_full_name(&self) -> Option<String> {
        let locale = "en-US".to_string();
        self.get_name(NameID::FullFontName, &locale)
            .ok()
            .filter(|name| !name.trim().is_empty())
            .or_else(|| {
                self.get_name(NameID::PostScriptName, &locale)
                    .ok()
                    .filter(|name| !name.trim().is_empty())
            })
    }

    pub(crate) fn face_weight_class(&self) -> u16 {
        let os2 = if self.current_font == 0 {
            self.os2.as_ref()
        } else {
            self.more_fonts[self.current_font - 1].os2.as_ref()
        };
        os2.map(|os2| os2.weight_class()).unwrap_or(400)
    }

    pub(crate) fn face_width_class(&self) -> u16 {
        let os2 = if self.current_font == 0 {
            self.os2.as_ref()
        } else {
            self.more_fonts[self.current_font - 1].os2.as_ref()
        };
        os2.map(|os2| os2.width_class()).unwrap_or(5)
    }

    pub(crate) fn face_is_italic(&self) -> bool {
        let head_mac_style = if self.current_font == 0 {
            self.head.as_ref().map(|head| head.mac_style)
        } else {
            self.more_fonts[self.current_font - 1]
                .head
                .as_ref()
                .map(|head| head.mac_style)
        }
        .unwrap_or(0);
        if head_mac_style & 0x0002 == 0x0002 {
            return true;
        }

        let post_italic_angle = if self.current_font == 0 {
            self.post.as_ref().map(|post| post.italic_angle)
        } else {
            self.more_fonts[self.current_font - 1]
                .post
                .as_ref()
                .map(|post| post.italic_angle)
        }
        .unwrap_or(0);
        if post_italic_angle != 0 {
            return true;
        }

        let os2_selection = if self.current_font == 0 {
            self.os2.as_ref().map(|os2| os2.selection_flags())
        } else {
            self.more_fonts[self.current_font - 1]
                .os2
                .as_ref()
                .map(|os2| os2.selection_flags())
        }
        .unwrap_or(0);
        os2_selection & 0x0001 == 0x0001
    }

    #[cfg(debug_assertions)]
    pub fn get_name_raw(&self) -> String {
        let name = if self.current_font == 0 {
            self.name.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1]
                .name
                .as_ref()
                .unwrap()
        };
        name.to_string()
    }

    #[cfg(debug_assertions)]
    pub fn get_maxp_raw(&self) -> String {
        let maxp = if self.current_font == 0 {
            self.maxp.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1]
                .maxp
                .as_ref()
                .unwrap()
        };
        maxp.to_string()
    }

    #[cfg(debug_assertions)]
    pub fn get_header_raw(&self) -> String {
        let head = if self.current_font == 0 {
            self.head.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1]
                .head
                .as_ref()
                .unwrap()
        };
        head.to_string()
    }

    #[cfg(debug_assertions)]
    pub fn get_os2_raw(&self) -> String {
        let os2 = if self.current_font == 0 {
            self.os2.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1].os2.as_ref().unwrap()
        };
        os2.to_string()
    }

    #[cfg(debug_assertions)]
    pub fn get_hhea_raw(&self) -> String {
        let hhea = if self.current_font == 0 {
            self.hhea.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1]
                .hhea
                .as_ref()
                .unwrap()
        };
        hhea.to_string()
    }

    #[cfg(debug_assertions)]
    pub fn get_cmap_raw(&self) -> String {
        let cmap = if self.current_font == 0 {
            self.cmap.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1]
                .cmap
                .as_ref()
                .unwrap()
        };
        let encodings = &cmap.cmap_encodings;
        let mut string = String::new();
        for encoding in encodings.as_ref().iter() {
            string += &format!(
                "Encording Record\n{}\n",
                encoding.encoding_record.to_string()
            );
            string += &format!(
                "Subtable\n{}\n",
                encoding.cmap_subtable.get_part_of_string(10)
            );
        }
        string
    }

    #[cfg(debug_assertions)]
    pub fn get_sbix_raw(&self) -> String {
        let sbix = if self.current_font == 0 {
            if let Some(sbix) = &self.sbix {
                sbix
            } else {
                return "sbix is none".to_string();
            }
        } else {
            let sbix = self.more_fonts[self.current_font - 1].sbix.as_ref();
            if let Some(sbix) = sbix {
                sbix
            } else {
                return "sbix is none".to_string();
            }
        };
        sbix.to_string()
    }

    #[cfg(debug_assertions)]
    pub fn get_svg_raw(&self) -> String {
        let svg = if self.current_font == 0 {
            if let Some(svg) = &self.svg {
                svg
            } else {
                return "svg is none".to_string();
            }
        } else {
            let svg = self.more_fonts[self.current_font - 1].svg.as_ref();
            if let Some(svg) = svg {
                svg
            } else {
                return "svg is none".to_string();
            }
        };
        svg.to_string()
    }

    #[cfg(debug_assertions)]
    pub fn get_post_raw(&self) -> String {
        let post = if self.current_font == 0 {
            self.post.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1]
                .post
                .as_ref()
                .unwrap()
        };
        post.to_string()
    }

    #[cfg(debug_assertions)]
    pub fn get_cpal_raw(&self) -> String {
        let cpal = if self.current_font == 0 {
            self.cpal.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1]
                .cpal
                .as_ref()
                .unwrap()
        };
        cpal.to_string()
    }

    #[cfg(debug_assertions)]
    pub fn get_colr_raw(&self) -> String {
        let colr = if self.current_font == 0 {
            self.colr.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1]
                .colr
                .as_ref()
                .unwrap()
        };
        colr.to_string()
    }
    #[cfg(debug_assertions)]
    #[cfg(feature = "layout")]
    pub fn get_vhea_raw(&self) -> String {
        let vhea = if self.current_font == 0 {
            self.vhea.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1]
                .vhea
                .as_ref()
                .unwrap()
        };
        vhea.to_string()
    }

    #[cfg(debug_assertions)]
    #[cfg(feature = "layout")]
    pub fn get_gdef_raw(&self) -> String {
        let gdef = if self.current_font == 0 {
            if self.gdef.is_none() {
                return "".to_string();
            }
            self.gdef.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1]
                .gdef
                .as_ref()
                .unwrap()
        };
        gdef.to_string()
    }

    #[cfg(debug_assertions)]
    #[cfg(feature = "layout")]
    pub fn get_gsub_raw(&self) -> String {
        let gsub = if self.current_font == 0 {
            self.gsub.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1]
                .gsub
                .as_ref()
                .unwrap()
        };
        gsub.to_string()
    }

    pub fn get_html_vert(
        &self,
        string: &str,
        fontsize: f64,
        fontunit: &str,
    ) -> Result<String, Error> {
        let mut html = String::new();
        html += "<html>\n";
        html += "<head>\n";
        html += "<meta charset=\"UTF-8\">\n";
        html += "<title>fontreader</title>\n";
        html += "<style>body {writing-mode: vertical-rl; }</style>\n";
        html += "</head>\n";
        html += "<body>\n";
        let mut svgs = Vec::new();
        for unit in Self::parse_text_units(string) {
            self.push_svg_html_unit(&mut svgs, unit, fontsize, fontunit, true)?;
        }

        for svg in svgs {
            html += &svg;
        }
        html += "</body>\n";
        html += "</html>\n";
        Ok(html)
    }

    pub fn get_html(&self, string: &str, fontsize: f64, fontunit: &str) -> Result<String, Error> {
        let mut html = String::new();
        html += "<html>\n";
        html += "<head>\n";
        html += "<meta charset=\"UTF-8\">\n";
        html += "<title>fontreader</title>\n";
        html += "</head>\n";
        html += "<body>\n";
        let mut svgs = Vec::new();
        for unit in Self::parse_text_units(string) {
            self.push_svg_html_unit(&mut svgs, unit, fontsize, fontunit, false)?;
        }
        for svg in svgs {
            html += &svg;
        }
        html += "</body>\n";
        html += "</html>\n";
        Ok(html)
    }

    pub fn get_info(&self) -> Result<String, Error> {
        let mut string = String::new();
        if self.name.is_none() {
            return Err(Error::new(
                std::io::ErrorKind::Other,
                "name table is none".to_string(),
            ));
        }
        let name = self.name.as_ref().unwrap();
        let font_famiry = name.get_family_name();
        let subfamily_name = name.get_subfamily_name();
        string += &format!("Font famiry: {} {}\n", font_famiry, subfamily_name);
        for more_font in self.more_fonts.iter() {
            if more_font.name.is_none() {
                continue;
            }
            let name = more_font.name.as_ref().unwrap();
            let font_famiry = name.get_family_name();
            let subfamily_name = name.get_subfamily_name();
            string += &format!("Font famiry: {} {}\n", font_famiry, subfamily_name);
        }
        Ok(string)
    }

    pub fn get_font_count(&self) -> usize {
        self.more_fonts.len() + 1
    }

    pub fn get_font_number(&self) -> usize {
        self.current_font
    }

    pub fn set_font(&mut self, number: usize) -> Result<(), String> {
        if number <= self.more_fonts.len() {
            self.current_font = number;
            Ok(())
        } else {
            Err("font number is out of range".to_owned())
        }
    }
}

#[derive(Debug, Clone)]
pub struct HorizontalLayout {
    pub lsb: isize,
    pub advance_width: isize,
    pub accender: isize,
    pub descender: isize,
    pub line_gap: isize,
    #[allow(dead_code)]
    pub(crate) hhea: HHEA,
}

#[derive(Debug, Clone)]
pub struct VerticalLayout {
    pub tsb: isize,
    pub advance_height: isize,
    pub accender: isize,
    pub descender: isize,
    pub line_gap: isize,
    #[allow(dead_code)]
    pub(crate) vhea: VHEA,
}

#[derive(Debug, Clone)]
struct Pointer {
    pub(crate) offset: u32,
    pub(crate) length: u32,
}

fn font_load_from_file(filename: &PathBuf) -> Result<Font, Error> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = filename;
        return Err(Error::new(
            ErrorKind::Unsupported,
            "file font loading is not supported on wasm32",
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let fontdata = std::fs::read(filename)?;
        Font::get_font_from_buffer(&fontdata)
    }
}

fn path_commands_to_svg_path(commands: &[PathCommand]) -> String {
    let mut d = String::new();
    for command in commands {
        match command {
            PathCommand::MoveTo { x, y } => d += &format!("M{} {} ", x, y),
            PathCommand::LineTo { x, y } => d += &format!("L{} {} ", x, y),
            PathCommand::QuadTo { cx, cy, x, y } => d += &format!("Q{} {} {} {} ", cx, cy, x, y),
            PathCommand::ClosePath => d += "Z ",
        }
    }
    d.trim_end().to_string()
}

fn path_command_bounds(commands: &[PathCommand]) -> (f64, f64, f64, f64) {
    let mut min_x = 0.0;
    let mut min_y = 0.0;
    let mut max_x = 0.0;
    let mut max_y = 0.0;
    let mut has_point = false;

    let mut add_point = |x: f64, y: f64| {
        if !has_point {
            min_x = x;
            min_y = y;
            max_x = x;
            max_y = y;
            has_point = true;
        } else {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    };

    for command in commands {
        match command {
            PathCommand::MoveTo { x, y } | PathCommand::LineTo { x, y } => add_point(*x, *y),
            PathCommand::QuadTo { cx, cy, x, y } => {
                add_point(*cx, *cy);
                add_point(*x, *y);
            }
            PathCommand::ClosePath => {}
        }
    }

    (min_x, min_y, max_x, max_y)
}

fn bitmap_glyph_to_svg_image(glyph: &GlyphCommands, bitmap: &BitmapGlyphCommands) -> String {
    let mime = match bitmap.format {
        BitmapGlyphFormat::Png => "image/png",
        BitmapGlyphFormat::Jpeg => "image/jpeg",
    };
    let encoded = general_purpose::STANDARD.encode(&bitmap.data);
    format!(
        "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" href=\"data:{};base64,{}\"/>",
        glyph.origin_x + bitmap.offset_x,
        glyph.origin_y + bitmap.offset_y,
        bitmap.width,
        bitmap.height,
        mime,
        encoded
    )
}

fn glyph_baseline_shift(layout: &FontLayout) -> f32 {
    match layout {
        FontLayout::Horizontal(layout) => (layout.accender + layout.line_gap) as f32,
        FontLayout::Vertical(layout) => (layout.accender - layout.descender) as f32,
        FontLayout::Unknown => 0.0,
    }
}

fn transform_glyf_commands(
    commands: &[PathCommand],
    layout: &FontLayout,
    scale_x: f32,
    scale_y: f32,
) -> Vec<DrawCommand> {
    let baseline_shift = glyph_baseline_shift(layout) as f64;
    commands
        .iter()
        .map(|command| match command {
            PathCommand::MoveTo { x, y } => {
                DrawCommand::MoveTo(*x as f32 * scale_x, (*y - baseline_shift) as f32 * scale_y)
            }
            PathCommand::LineTo { x, y } => {
                DrawCommand::Line(*x as f32 * scale_x, (*y - baseline_shift) as f32 * scale_y)
            }
            PathCommand::QuadTo { cx, cy, x, y } => DrawCommand::Bezier(
                (
                    *cx as f32 * scale_x,
                    (*cy - baseline_shift) as f32 * scale_y,
                ),
                (*x as f32 * scale_x, (*y - baseline_shift) as f32 * scale_y),
            ),
            PathCommand::ClosePath => DrawCommand::Close,
        })
        .collect()
}

fn transform_cff_commands(
    commands: &[DrawCommand],
    scale_x: f32,
    scale_y: f32,
) -> Vec<DrawCommand> {
    commands
        .iter()
        .map(|command| match command {
            DrawCommand::MoveTo(x, y) => DrawCommand::MoveTo(*x * scale_x, *y * scale_y),
            DrawCommand::Line(x, y) => DrawCommand::Line(*x * scale_x, *y * scale_y),
            DrawCommand::Bezier((cx, cy), (x, y)) => {
                DrawCommand::Bezier((*cx * scale_x, *cy * scale_y), (*x * scale_x, *y * scale_y))
            }
            DrawCommand::CubicBezier((xa, ya), (xb, yb), (xc, yc)) => DrawCommand::CubicBezier(
                (*xa * scale_x, *ya * scale_y),
                (*xb * scale_x, *yb * scale_y),
                (*xc * scale_x, *yc * scale_y),
            ),
            DrawCommand::Close => DrawCommand::Close,
        })
        .collect()
}

fn font_metrics_from_layout(layout: &FontLayout, scale_y: f32) -> DrawFontMetrics {
    match layout {
        FontLayout::Horizontal(layout) => DrawFontMetrics {
            ascent: layout.accender as f32 * scale_y,
            descent: (-layout.descender) as f32 * scale_y,
            line_gap: layout.line_gap as f32 * scale_y,
            flow: GlyphFlow::Horizontal,
        },
        FontLayout::Vertical(layout) => DrawFontMetrics {
            ascent: layout.accender as f32 * scale_y,
            descent: (-layout.descender) as f32 * scale_y,
            line_gap: layout.line_gap as f32 * scale_y,
            flow: GlyphFlow::Vertical,
        },
        FontLayout::Unknown => DrawFontMetrics {
            ascent: 0.0,
            descent: 0.0,
            line_gap: 0.0,
            flow: GlyphFlow::Horizontal,
        },
    }
}

fn glyph_metrics_from_layout(layout: &FontLayout, scale_x: f32, scale_y: f32) -> DrawGlyphMetrics {
    match layout {
        FontLayout::Horizontal(layout) => DrawGlyphMetrics {
            advance_x: layout.advance_width as f32 * scale_x,
            advance_y: 0.0,
            bearing_x: layout.lsb as f32 * scale_x,
            bearing_y: layout.accender as f32 * scale_y,
            bounds: None,
        },
        FontLayout::Vertical(layout) => DrawGlyphMetrics {
            advance_x: 0.0,
            advance_y: layout.advance_height as f32 * scale_y,
            bearing_x: 0.0,
            bearing_y: layout.accender as f32 * scale_y,
            bounds: None,
        },
        FontLayout::Unknown => DrawGlyphMetrics::default(),
    }
}

fn glyph_layers_bounds(layers: &[GlyphLayer]) -> Option<GlyphBounds> {
    let mut bounds = None;

    for layer in layers {
        match layer {
            GlyphLayer::Path(path) => {
                for command in path.commands.iter() {
                    match command {
                        DrawCommand::MoveTo(x, y) | DrawCommand::Line(x, y) => {
                            extend_bounds(&mut bounds, *x + path.offset_x, *y + path.offset_y);
                        }
                        DrawCommand::Bezier((cx, cy), (x, y)) => {
                            extend_bounds(&mut bounds, *cx + path.offset_x, *cy + path.offset_y);
                            extend_bounds(&mut bounds, *x + path.offset_x, *y + path.offset_y);
                        }
                        DrawCommand::CubicBezier((xa, ya), (xb, yb), (xc, yc)) => {
                            extend_bounds(&mut bounds, *xa + path.offset_x, *ya + path.offset_y);
                            extend_bounds(&mut bounds, *xb + path.offset_x, *yb + path.offset_y);
                            extend_bounds(&mut bounds, *xc + path.offset_x, *yc + path.offset_y);
                        }
                        DrawCommand::Close => {}
                    }
                }
            }
            GlyphLayer::Raster(raster) => {
                if let (Some(width), Some(height)) = (raster.width, raster.height) {
                    extend_bounds(&mut bounds, raster.offset_x, raster.offset_y);
                    extend_bounds(
                        &mut bounds,
                        raster.offset_x + width as f32,
                        raster.offset_y + height as f32,
                    );
                }
            }
        }
    }

    bounds
}

fn extend_bounds(bounds: &mut Option<GlyphBounds>, x: f32, y: f32) {
    if let Some(bounds) = bounds.as_mut() {
        bounds.min_x = bounds.min_x.min(x);
        bounds.min_y = bounds.min_y.min(y);
        bounds.max_x = bounds.max_x.max(x);
        bounds.max_y = bounds.max_y.max(y);
    } else {
        *bounds = Some(GlyphBounds {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        });
    }
}

#[cfg(debug_assertions)]
#[allow(dead_code)]
fn font_debug(_font: &Font) {
    // create or open file
    let filename = "test/font.txt";
    let file = match File::create(filename) {
        Ok(it) => it,
        Err(_) => File::open("test/font.txt").unwrap(),
    };
    let mut writer = BufWriter::new(file);

    let encoding_records = &_font.cmap.as_ref().unwrap().get_encoding_engine();
    writeln!(&mut writer, "{}", _font.cmap.as_ref().unwrap().cmap).unwrap();
    for i in 0..encoding_records.len() {
        writeln!(&mut writer, "{} {}", i, encoding_records[i].to_string()).unwrap();
    }
    writeln!(&mut writer, "{}", &_font.head.as_ref().unwrap().to_string()).unwrap();
    writeln!(&mut writer, "{}", &_font.hhea.as_ref().unwrap().to_string()).unwrap();
    writeln!(&mut writer, "{}", &_font.maxp.as_ref().unwrap().to_string()).unwrap();
    writeln!(&mut writer, "{}", &_font.hmtx.as_ref().unwrap().to_string()).unwrap();
    if _font.os2.is_some() {
        writeln!(&mut writer, "{}", &_font.os2.as_ref().unwrap().to_string()).unwrap();
    }
    if _font.post.is_some() {
        writeln!(&mut writer, "{}", &_font.post.as_ref().unwrap().to_string()).unwrap();
    }
    if _font.name.is_some() {
        writeln!(&mut writer, "{}", &_font.name.as_ref().unwrap().to_string()).unwrap();
    }

    if _font.loca.is_some() {
        writeln!(&mut writer, "{}", &_font.loca.as_ref().unwrap().to_string()).unwrap();
    } else {
        writeln!(&mut writer, "loca is none. it is not glyf font.").unwrap();
        return;
    }
    if _font.cpal.is_some() {
        writeln!(&mut writer, "{}", &_font.cpal.as_ref().unwrap().to_string()).unwrap();
    }
    if _font.colr.is_some() {
        writeln!(&mut writer, "{}", &_font.colr.as_ref().unwrap().to_string()).unwrap();
    }

    writeln!(&mut writer, "long cmap -> griph").unwrap();
    let cmap_encodings = &_font.cmap.as_ref().unwrap().clone();
    let glyf = _font.glyf.as_ref().unwrap();
    for i in 0x0020..0x0ff {
        let pos = cmap_encodings.get_glyph_position(i);
        let glyph = glyf.get_glyph(pos as usize).unwrap();
        let layout = _font.get_layout(pos as usize, false);
        let svg = glyph.to_svg(32.0, "pt", &layout, 0.0, 0.0);
        let ch = char::from_u32(i).unwrap();
        writeln!(&mut writer, "{}:{:04} ", ch, pos).unwrap();
        writeln!(&mut writer, "{}", glyph.to_string()).unwrap();
        writeln!(&mut writer, "{}:{:?}", i, layout).unwrap();
        writeln!(&mut writer, "{}", svg).unwrap();
    }
    writeln!(&mut writer).unwrap();
    for i in 0x4e00..0x4eff {
        if i as u32 % 16 == 0 {
            writeln!(&mut writer).unwrap();
        }
        let pos = cmap_encodings.get_glyph_position(i as u32);
        let glyph = glyf.get_glyph(pos as usize).unwrap();
        let layout = _font.get_layout(pos as usize, false);
        let svg = glyph.to_svg(100.0, "px", &layout, 0.0, 0.0);
        let ch = char::from_u32(i as u32).unwrap();
        write!(&mut writer, "{}:{:04} ", ch, pos).unwrap();
        writeln!(&mut writer, "{}", svg).unwrap();
    }
    writeln!(&mut writer).unwrap();
    let i = 0x2a6b2;
    let pos = cmap_encodings.get_glyph_position(i as u32);
    let ch = char::from_u32(i as u32).unwrap();
    writeln!(&mut writer, "{}:{:04} ", ch, pos).unwrap();
}

fn font_load<R: BinaryReader>(file: &mut R) -> Result<Font, Error> {
    match fontheader::get_font_type(file)? {
        fontheader::FontHeaders::OTF(header) => {
            let font = from_opentype(file, &header);
            #[cfg(debug_assertions)]
            {
                // font_debug(font.as_ref().unwrap());
            }
            font
        }
        fontheader::FontHeaders::TTC(header) => {
            let num_fonts = header.num_fonts;
            let font_collection = header.font_collection.as_ref();
            let table = &font_collection[0];
            let mut font = from_opentype(file, table);
            #[cfg(debug_assertions)]
            {
                // font_debug(font.as_ref().unwrap());
            }

            let mut fonts = Vec::new();
            for i in 1..num_fonts {
                let table = &font_collection[i as usize];
                let font = from_opentype(file, table);
                match font.is_ok() {
                    true => {
                        fonts.push(font.unwrap());
                    }
                    false => (),
                }
            }
            if let Ok(font) = font.as_mut() {
                font.more_fonts = Box::new(fonts);
                #[cfg(debug_assertions)]
                {
                    //    font_debug(font.as_ref().unwrap());
                }
            }
            font
        }
        fontheader::FontHeaders::WOFF(header) => {
            let mut font = Font::empty();
            font.font_type = fontheader::FontHeaders::WOFF(header.clone());
            let woff = crate::woff::WOFF::from(file, header)?;

            let mut hmtx_table = None;
            let mut loca_table = None;
            let mut glyf_table = None;
            let mut sbix_table = None;
            let mut vmtx_table = None;
            for table in woff.tables {
                let tag: [u8; 4] = [
                    (table.tag >> 24) as u8,
                    (table.tag >> 16) as u8,
                    (table.tag >> 8) as u8,
                    table.tag as u8,
                ];
                // println!("tag: {}", crate::util::u32_to_string(table.tag));
                match &tag {
                    b"cmap" => {
                        let mut reader = BytesReader::new(&table.data);
                        let cmap_encodings =
                            CmapEncodings::new(&mut reader, 0, table.data.len() as u32)?;
                        font.cmap = Some(cmap_encodings);
                    }
                    b"head" => {
                        let mut reader = BytesReader::new(&table.data);
                        let head = head::HEAD::new(&mut reader, 0, table.data.len() as u32)?;
                        font.head = Some(head);
                    }
                    b"OS/2" => {
                        let mut reader = BytesReader::new(&table.data);
                        let os2 = os2::OS2::new(&mut reader, 0, table.data.len() as u32)?;
                        font.os2 = Some(os2);
                    }
                    b"hhea" => {
                        let mut reader = BytesReader::new(&table.data);
                        let hhea = hhea::HHEA::new(&mut reader, 0, table.data.len() as u32)?;
                        font.hhea = Some(hhea);
                    }
                    b"maxp" => {
                        let mut reader = BytesReader::new(&table.data);
                        let maxp = maxp::MAXP::new(&mut reader, 0, table.data.len() as u32)?;
                        font.maxp = Some(maxp);
                    }
                    b"hmtx" => {
                        hmtx_table = Some(table);
                    }
                    b"name" => {
                        let mut reader = BytesReader::new(&table.data);
                        let name = name::NAME::new(&mut reader, 0, table.data.len() as u32)?;
                        let name_table = name::NameTable::new(&name);
                        font.name = Some(name);
                        font.name_table = Some(name_table);
                    }
                    b"post" => {
                        let mut reader = BytesReader::new(&table.data);
                        let post = post::POST::new(&mut reader, 0, table.data.len() as u32)?;
                        font.post = Some(post);
                    }
                    b"loca" => {
                        loca_table = Some(table);
                    }
                    b"glyf" => {
                        glyf_table = Some(table);
                    }
                    b"COLR" => {
                        let mut reader = BytesReader::new(&table.data);
                        let colr = colr::COLR::new(&mut reader, 0, table.data.len() as u32)?;
                        font.colr = Some(colr);
                    }
                    b"CPAL" => {
                        let mut reader = BytesReader::new(&table.data);
                        let cpal = cpal::CPAL::new(&mut reader, 0, table.data.len() as u32)?;
                        font.cpal = Some(cpal);
                    }
                    b"sbix" => {
                        sbix_table = Some(table);
                    }
                    b"SVG " => {
                        let mut reader = BytesReader::new(&table.data);
                        let svg = svg::SVG::new(&mut reader, 0, table.data.len() as u32)?;
                        font.svg = Some(svg);
                    }
                    #[cfg(feature = "cff")]
                    b"CFF " => {
                        let mut reader = BytesReader::new(&table.data);
                        let cff = cff::CFF::new(&mut reader, 0, table.data.len() as u32);
                        font.cff = Some(cff.unwrap());
                    }
                    #[cfg(feature = "layout")]
                    b"GPOS" => {
                        let mut reader = BytesReader::new(&table.data);
                        let gpos = gpos::GPOS::new(&mut reader, 0, table.data.len() as u32)?;
                        font.gpos = Some(gpos);
                    }
                    #[cfg(feature = "layout")]
                    b"GSUB" => {
                        let mut reader = BytesReader::new(&table.data);
                        let gsub = gsub::GSUB::new(&mut reader, 0, table.data.len() as u32)?;
                        font.gsub = Some(gsub);
                    }
                    #[cfg(feature = "layout")]
                    b"GDEF" => {
                        let mut reader = BytesReader::new(&table.data);
                        let gdef = gdef::GDEF::new(&mut reader, 0, table.data.len() as usize)?;
                        font.gdef = Some(gdef);
                    }
                    b"vhea" => {
                        let mut reader = BytesReader::new(&table.data);
                        let vhea = vhea::VHEA::new(&mut reader, 0, table.data.len() as u32)?;
                        font.vhea = Some(vhea);
                    }
                    b"vmtx" => {
                        vmtx_table = Some(table);
                    }
                    _ => {}
                }
            }
            let mut reader = BytesReader::new(&hmtx_table.as_ref().unwrap().data);
            let hmtx = hmtx::HMTX::new(
                &mut reader,
                0,
                hmtx_table.as_ref().unwrap().data.len() as u32,
                font.hhea.as_ref().unwrap().number_of_hmetrics,
                font.maxp.as_ref().unwrap().num_glyphs,
            )?;
            font.hmtx = Some(hmtx);
            if let Some(vmtx_table) = vmtx_table {
                let mut reader = BytesReader::new(&vmtx_table.data);
                let vmtx = vmtx::VMTX::new(
                    &mut reader,
                    0,
                    vmtx_table.data.len() as u32,
                    font.vhea.as_ref().unwrap().number_of_vmetrics,
                    font.maxp.as_ref().unwrap().num_glyphs,
                )?;
                font.vmtx = Some(vmtx);
            }
            let mut reader = BytesReader::new(&loca_table.as_ref().unwrap().data);
            let index_to_loc_format = font.head.as_ref().unwrap().index_to_loc_format as usize;
            let loca = loca::LOCA::new_by_size(
                &mut reader,
                0,
                loca_table.as_ref().unwrap().data.len() as u32,
                index_to_loc_format,
            )?;
            font.loca = Some(loca);
            let mut reader = BytesReader::new(&glyf_table.as_ref().unwrap().data);
            let glyf = glyf::GLYF::new(
                &mut reader,
                0,
                glyf_table.as_ref().unwrap().data.len() as u32,
                font.loca.as_ref().unwrap(),
            );
            font.glyf = Some(glyf);

            if let Some(sbix_table) = sbix_table {
                let mut reader = BytesReader::new(&sbix_table.data);
                let num_glyphs = font.maxp.as_ref().unwrap().num_glyphs as u32;
                let sbix = sbix::SBIX::new(&mut reader, 0, sbix_table.data.len() as u32, num_glyphs)?;
                font.sbix = Some(sbix);
            }
            #[cfg(debug_assertions)]
            {
                // font_debug(&font);
            }
            Ok(font)
        }
        fontheader::FontHeaders::WOFF2(_) => todo!(),
        fontheader::FontHeaders::Unknown => {
            //todo!(),
            Err(Error::new(
                std::io::ErrorKind::Other,
                "Unknown font type".to_string(),
            ))
        }
    }
}

fn from_opentype<R: BinaryReader>(file: &mut R, header: &OTFHeader) -> Result<Font, Error> {
    let mut font = Font::empty();
    font.font_type = fontheader::FontHeaders::OTF(header.clone());

    let records = header.table_records.as_ref();

    for record in records.iter() {
        let tag: [u8; 4] = record.table_tag.to_be_bytes();
        match &tag {
            b"cmap" => {
                let cmap_encodings = CmapEncodings::new(file, record.offset, record.length)?;
                font.cmap = Some(cmap_encodings);
            }
            b"head" => {
                let head = head::HEAD::new(file, record.offset, record.length)?;
                font.head = Some(head);
            }
            b"hhea" => {
                let hhea = hhea::HHEA::new(file, record.offset, record.length)?;
                font.hhea = Some(hhea);
            }
            b"hmtx" => {
                let htmx_pos = Pointer {
                    offset: record.offset,
                    length: record.length,
                };
                font.hmtx_pos = Some(htmx_pos);
            }
            b"maxp" => {
                let maxp = maxp::MAXP::new(file, record.offset, record.length)?;
                font.maxp = Some(maxp);
            }
            b"name" => {
                let name = name::NAME::new(file, record.offset, record.length)?;
                let name_table = name::NameTable::new(&name);
                font.name = Some(name);
                font.name_table = Some(name_table);
            }
            b"OS/2" => {
                let os2 = os2::OS2::new(file, record.offset, record.length)?;
                font.os2 = Some(os2);
            }
            b"post" => {
                let post = post::POST::new(file, record.offset, record.length)?;
                font.post = Some(post);
            }
            b"loca" => {
                let loca_pos = Pointer {
                    offset: record.offset,
                    length: record.length,
                };
                font.loca_pos = Some(loca_pos);
            }
            b"glyf" => {
                let glyf_pos = Pointer {
                    offset: record.offset,
                    length: record.length,
                };
                font.glyf_pos = Some(glyf_pos);
            }
            b"COLR" => {
                let colr = colr::COLR::new(file, record.offset, record.length)?;
                font.colr = Some(colr);
            }
            b"CPAL" => {
                let cpal = cpal::CPAL::new(file, record.offset, record.length)?;
                font.cpal = Some(cpal);
            }
            b"sbix" => {
                let sbix_pos = Pointer {
                    offset: record.offset,
                    length: record.length,
                };
                font.sbix_pos = Some(sbix_pos);
            }
            b"SVG " => {
                let svg = svg::SVG::new(file, record.offset, record.length)?;
                font.svg = Some(svg);
            }
            #[cfg(feature = "cff")]
            b"CFF " => {
                let cff = cff::CFF::new(file, record.offset, record.length);
                font.cff = Some(cff.unwrap());
            }
            #[cfg(feature = "layout")]
            b"GPOS" => {
                let gpos = gpos::GPOS::new(file, record.offset, record.length)?;
                font.gpos = Some(gpos);
            }
            #[cfg(feature = "layout")]
            b"GSUB" => {
                let gsub = gsub::GSUB::new(file, record.offset, record.length)?;
                font.gsub = Some(gsub);
                #[cfg(debug_assertions)]
                {
                    // println!("{}", &font.gsub.as_ref().unwrap().to_string());
                }
            }
            #[cfg(feature = "layout")]
            b"GDEF" => {
                let gdef = gdef::GDEF::new(file, record.offset as u64, record.length as usize)?;
                font.gdef = Some(gdef);
                #[cfg(debug_assertions)]
                {
                    // println!("{}", &font.gdef.as_ref().unwrap().to_string());
                }
            }
            #[cfg(feature = "layout")]
            b"vhea" => {
                let vhea = vhea::VHEA::new(file, record.offset, record.length)?;
                font.vhea = Some(vhea);
            }
            #[cfg(feature = "layout")]
            b"vmtx" => {
                let vmtx_pos = Pointer {
                    offset: record.offset,
                    length: record.length,
                };
                font.vmtx_pos = Some(vmtx_pos);
            }
            _ => {
                debug_assert!(true, "Unknown table tag")
            }
        }
    }

    let num_glyphs = font.maxp.as_ref().unwrap().num_glyphs;
    let number_of_hmetrics = font.hhea.as_ref().unwrap().number_of_hmetrics;
    let offset = font.hmtx_pos.as_ref().unwrap().offset;
    let length = font.hmtx_pos.as_ref().unwrap().length;

    let hmtx = hmtx::HMTX::new(file, offset, length, number_of_hmetrics, num_glyphs)?;
    font.hmtx = Some(hmtx);

    if font.vmtx_pos.is_some() {
        let number_of_vmetrics = font.vhea.as_ref().unwrap().number_of_vmetrics;
        let offset = font.vmtx_pos.as_ref().unwrap().offset;
        let length = font.vmtx_pos.as_ref().unwrap().length;
        let vmtx = vmtx::VMTX::new(file, offset, length, number_of_vmetrics, num_glyphs)?;
        font.vmtx = Some(vmtx);
    }

    if let Some(offset) = font.loca_pos.as_ref() {
        let length = font.loca_pos.as_ref().unwrap().length;
        let index_to_loc_format = font.head.as_ref().unwrap().index_to_loc_format as usize;
        let loca = loca::LOCA::new_by_size(file, offset.offset, length, index_to_loc_format)?;
        font.loca = Some(loca);
        let offset = font.glyf_pos.as_ref().unwrap().offset;
        let length = font.glyf_pos.as_ref().unwrap().length;
        let loca = font.loca.as_ref().unwrap();
        let glyf = glyf::GLYF::new(file, offset, length, loca);
        font.glyf = Some(glyf);
    }
    if let Some(offset) = font.sbix_pos.as_ref() {
        let sbix = sbix::SBIX::new(file, offset.offset, offset.length, num_glyphs as u32)?;
        font.sbix = Some(sbix);
    }

    if font.cmap.is_none() {
        debug_assert!(true, "No cmap table");
        return Err(Error::new(
            std::io::ErrorKind::Other,
            "No cmap table".to_string(),
        ));
    }
    if font.head.is_none() {
        debug_assert!(true, "No head table");
        return Err(Error::new(
            std::io::ErrorKind::Other,
            "No head table".to_string(),
        ));
    }
    if font.hhea.is_none() {
        debug_assert!(true, "No hhea table");
        return Err(Error::new(
            std::io::ErrorKind::Other,
            "No hhea table".to_string(),
        ));
    }
    if font.hmtx.is_none() {
        debug_assert!(true, "No hmtx table");
        return Err(Error::new(
            std::io::ErrorKind::Other,
            "No hmtx table".to_string(),
        ));
    }
    if font.maxp.is_none() {
        debug_assert!(true, "No maxp table");
        return Err(Error::new(
            std::io::ErrorKind::Other,
            "No maxp table".to_string(),
        ));
    }
    if font.name.is_none() {
        debug_assert!(true, "No name table");
        return Err(Error::new(
            std::io::ErrorKind::Other,
            "No name table".to_string(),
        ));
    }

    Ok(font)
}
