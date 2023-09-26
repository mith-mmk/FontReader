use bin_rs::reader::{BinaryReader, BytesReader, StreamReader};
use std::collections::HashMap;
use std::io::BufReader;
use std::io::Error;
use std::{fs::File, path::PathBuf};

use crate::fontheader;
use crate::opentype::color::sbix;
use crate::opentype::color::svg;
use crate::opentype::color::{colr, cpal};
#[cfg(feature = "layout")]
use crate::opentype::extentions::gsub;
use crate::opentype::platforms::PlatformID;
use crate::opentype::requires::cmap::CmapEncodings;
use crate::opentype::requires::hmtx::LongHorMetric;
use crate::opentype::requires::name::NameID;
use crate::opentype::requires::*;
use crate::opentype::{outline::*, OTFHeader};

#[cfg(debug_assertions)]
use std::io::{BufWriter, Write};

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
    format: GlyphFormat,
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
    pub(crate) gsub: Option<gsub::GSUB>,
    pub(crate) svg: Option<svg::SVG>,
    pub(crate) sbix: Option<sbix::SBIX>,
    hmtx_pos: Option<Pointer>,
    loca_pos: Option<Pointer>, // OpenType font, CFF/CFF2 none
    glyf_pos: Option<Pointer>, // OpenType font, CFF/CFF2 none
    sbix_pos: Option<Pointer>,
    pub(crate) more_fonts: Box<Vec<Font>>,
    current_font: usize,
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
            gsub: None,
            sbix: None,
            svg: None,
            hmtx_pos: None,
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
        }
    }

    pub fn get_glyph_from_id(&self, glyph_id: usize) -> GriphData {
        let glyf = if self.current_font == 0 {
            self.glyf.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1]
                .glyf
                .as_ref()
                .unwrap()
        };

        let glyph = glyf.get_glyph(glyph_id).unwrap();
        let layout: HorizontalLayout = self.get_horizontal_layout(glyph_id);
        let open_type_glyph = OpenTypeGlyph {
            layout: FontLayout::Horizontal(layout),
            glyph: FontData::Glyph(glyph.clone()),
        };

        GriphData {
            glyph_id,
            format: GlyphFormat::OpenTypeGlyph,
            open_type_glyf: Some(open_type_glyph),
        }
    }

    pub fn get_glyph_with_uvs(&self, ch: char, vs: char) -> GriphData {
        let code = ch as u32;
        let vs = vs as u32;

        #[cfg(feature = "cff")]
        {
            if self.cff.is_some() {
                let (cmap, cff) = if self.current_font == 0 {
                    (self.cmap.as_ref().unwrap(), self.cff.as_ref().unwrap())
                } else {
                    (
                        self.more_fonts[self.current_font - 1]
                            .cmap
                            .as_ref()
                            .unwrap(),
                        self.more_fonts[self.current_font - 1].cff.as_ref().unwrap(),
                    )
                };
                let glyph_id = cmap.get_glyph_position_from_uvs(code, vs) as usize;
                let layout = self.get_horizontal_layout(glyph_id as usize);

                let string = cff.to_code(glyph_id, &layout);
                // println!("cff string: {}", string);
                let open_type_glyf = Some(OpenTypeGlyph {
                    layout: FontLayout::Horizontal(layout),
                    glyph: FontData::CFF(string.as_bytes().to_vec()),
                });

                return GriphData {
                    glyph_id,
                    format: GlyphFormat::CFF,
                    open_type_glyf: open_type_glyf,
                };
            }
        }

        let (cmap, glyf) = if self.current_font == 0 {
            (self.cmap.as_ref().unwrap(), self.glyf.as_ref().unwrap())
        } else {
            (
                self.more_fonts[self.current_font - 1]
                    .cmap
                    .as_ref()
                    .unwrap(),
                self.more_fonts[self.current_font - 1]
                    .glyf
                    .as_ref()
                    .unwrap(),
            )
        };

        let pos = cmap.get_glyph_position_from_uvs(code, vs);
        //        let pos = cmap.get_glyph_position(code);
        let glyph = glyf.get_glyph(pos as usize).unwrap();
        let layout: HorizontalLayout = self.get_horizontal_layout(pos as usize);
        let open_type_glyph = OpenTypeGlyph {
            layout: FontLayout::Horizontal(layout),
            glyph: FontData::Glyph(glyph.clone()),
        };

        GriphData {
            glyph_id: pos as usize,
            format: GlyphFormat::OpenTypeGlyph,
            open_type_glyf: Some(open_type_glyph),
        }
    }

    pub fn get_glyph(&self, ch: char) -> GriphData {
        self.get_glyph_with_uvs(ch, '\u{0}')
    }

    pub fn get_svg_with_uvs(&self, ch: char, vs: char,fontsize: f64,fontunit: &str) -> Result<String, Error> {
        // svg ?
        // sbix ?
        // cff ?
        #[cfg(feature = "cff")]
        if let Some(cff) = self.cff.as_ref() {
            let gid = self.cmap.as_ref().unwrap().get_glyph_position(ch as u32) as usize;
            let layout = self.get_horizontal_layout(gid as usize);
            let string = cff.to_svg(gid,fontsize, fontunit, &layout, 0.0, 0.0);
            return Ok(string);
        }

        if self.glyf.is_none() {
            return Err(Error::new(
                std::io::ErrorKind::Other,
                "glyf is none".to_string(),
            ));
        }

        // utf-32
        let glyf_data = self.get_glyph_with_uvs(ch, vs);
        let pos = glyf_data.glyph_id;

        if let FontData::Glyph(glyph) = &glyf_data.open_type_glyf.as_ref().unwrap().glyph {
            let layout = &glyf_data.open_type_glyf.as_ref().unwrap().layout;
            let layout = match layout {
                FontLayout::Horizontal(layout) => layout,
                _ => panic!("not support vertical layout"),
            };
            if let Some(sbix) = self.sbix.as_ref() {
                let result = sbix.get_svg(pos as u32, fontsize, fontunit, &layout, 0.0, 0.0);
                if let Some(svg) = result {
                    let mut string = "".to_string();
                    #[cfg(debug_assertions)]
                    {
                        string += &format!("<!-- {} glyf id: {} -->", ch, pos);
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
                        string += &format!("<!-- {} glyf id: {} -->", ch, pos);
                        string += &format!(
                            "<!-- layout {} {} {} {} {} -->\n",
                            layout.lsb,
                            layout.advance_width,
                            layout.accender,
                            layout.descender,
                            layout.line_gap
                        );
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
                    return Ok(glyph.to_svg(fontsize, fontunit, &layout, 0.0, 0.0));
                }
                let mut string = glyph.get_svg_heder(fontsize, fontunit, &layout);
                #[cfg(debug_assertions)]
                {
                    string += &format!("\n<!-- {} glyf id: {} -->", ch, pos);
                }

                for layer in layers {
                    let glyf_id = layer.glyph_id as u32;
                    let glyf = glyf.get_glyph(glyf_id as usize).unwrap();
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
                    string += &glyf.get_svg_path(&layout);
                    string += "</g>\n";
                }
                string += "</svg>";
                Ok(string)
            } else {
                #[cfg(debug_assertions)]
                {
                    let string = glyph.to_svg(fontsize, fontunit, &layout, 0.0, 0.0);
                    return Ok(format!("<!-- {} glyf id: {} -->{}", ch, pos, string));
                }
                #[cfg(not(debug_assertions))]
                Ok(glyph.to_svg(fontsize, fontunit, &layout, 0.0, 0.0))
            }
        } else {
            return Err(Error::new(
                std::io::ErrorKind::Other,
                "glyf is none".to_string(),
            ));
        }
    }

    pub fn get_svg(&self, ch: char, fontsize: f64, fontunit: &str) -> Result<String, Error> {
        self.get_svg_with_uvs(ch, '\u{0}', fontsize, fontunit)
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


    pub fn get_html(&self, string: &str, fontsize: f64, fontunit: &str) -> Result<String, Error> {
        let mut html = String::new();
        html += "<html>\n";
        html += "<head>\n";
        html += "<meta charset=\"UTF-8\">\n";
        html += "<title>fontreader</title>\n";
        html += "</head>\n";
        html += "<body>\n";
        let mut svgs = Vec::new();
        for (i, ch) in string.chars().enumerate() {
            if ch == '\n' {
                svgs.push("<br>".to_string());
                continue;
            }
            if ch == '\r' {
                continue;
            }
            if ch == '\t' {
                svgs.push(
                    "<span style=\"width: 4em; display: inline-block;\"></span>\n".to_string(),
                );
                continue;
            }

            // variation selector 0xE0100 - 0xE01EF
            // https://www.unicode.org/reports/tr37/#VS
            if ch as u32 >= 0xfe00 && ch as u32 <= 0xfe0f
                || ch as u32 >= 0xE0100 && ch as u32 <= 0xE01EF
            {
                let ch0 = string.chars().nth(i - 1).unwrap();
                let svg = self.get_svg_with_uvs(ch0, ch, fontsize, fontunit)?;
                svgs.pop();
                svgs.push(svg);
            } else {
                let svg = self.get_svg(ch, fontsize, fontunit)?;
                svgs.push(svg);
            }
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

enum Layout {
    Horizontal(HorizontalLayout),
    Vertical(VerticalLayout),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct HorizontalLayout {
    pub lsb: isize,
    pub advance_width: isize,
    pub accender: isize,
    pub descender: isize,
    pub line_gap: isize,
}

#[derive(Debug, Clone)]
pub struct VerticalLayout {
    pub tsb: isize,
    pub advance_height: isize,
    pub accender: isize,
    pub descender: isize,
    pub line_gap: isize,
}

#[derive(Debug, Clone)]
struct Pointer {
    pub(crate) offset: u32,
    pub(crate) length: u32,
}

fn font_load_from_file(filename: &PathBuf) -> Result<Font, Error> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let mut reader = StreamReader::new(reader);
    font_load(&mut reader)
}

#[cfg(debug_assertions)]
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
        let layout = _font.get_horizontal_layout(pos as usize);
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
        let layout = _font.get_horizontal_layout(pos as usize);
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
    match fontheader::get_font_type(file) {
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
            let woff = crate::woff::WOFF::from(file, header);

            let mut hmtx_table = None;
            let mut loca_table = None;
            let mut glyf_table = None;
            let mut sbix_table = None;
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
                        println!("cmap");
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
                        print!("hmtx");
                        print!("{} ", table.data.len());
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
                    b"GSUB" => {
                        let mut reader = BytesReader::new(&table.data);
                        let gsub = gsub::GSUB::new(&mut reader, 0, table.data.len() as u32);
                        font.gsub = Some(gsub);
                    }
                    b"sbix" => {
                        sbix_table = Some(table);
                    }
                    _ => {
                        debug_assert!(true, "Unknown table tag")
                    }
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
                let sbix = sbix::SBIX::new(&mut reader, 0, num_glyphs)?;
                font.sbix = Some(sbix);
            }
            #[cfg(debug_assertions)]
            {
                font_debug(&font);
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
        #[cfg(debug_assertions)]
        {
            for i in 0..4 {
                let ch = tag[i] as char;
                print!("{}", ch);
            }
            println!("{:?}", tag);
        }

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
            b"GSUB" => {
                let gsub = gsub::GSUB::new(file, record.offset, record.length);
                font.gsub = Some(gsub);
                #[cfg(debug_assertions)]
                {
                    println!("{}", &font.gsub.as_ref().unwrap().to_string());
                }
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
        let sbix = sbix::SBIX::new(file, offset.offset, num_glyphs as u32)?;
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
