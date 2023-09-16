use bin_rs::reader::{BinaryReader, BytesReader, StreamReader};
use std::collections::HashMap;

use std::io::BufReader;
use std::{fs::File, path::PathBuf};

use crate::fontheader;
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
    pub(crate) open_type_glif: Option<OpenTypeGlyph>,
}

#[derive(Debug, Clone)]
pub struct OpenTypeGlyph {
    layout: FontLayout,
    glyph: Box<glyf::Glyph>,
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
    pub(crate) grif: Option<glyf::GLYF>, // openType font, CFF/CFF2 none
    #[cfg(feature = "cff")]
    pub(crate) cff: Option<cff::CFF>, // CFF font, openType none
    pub(crate) colr: Option<colr::COLR>,
    pub(crate) cpal: Option<cpal::CPAL>,
    #[cfg(feature = "layout")]
    pub(crate) gsub: Option<gsub::GSUB>,
    hmtx_pos: Option<Pointer>,
    loca_pos: Option<Pointer>, // OpenType font, CFF/CFF2 none
    glyf_pos: Option<Pointer>, // OpenType font, CFF/CFF2 none
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
            grif: None,
            #[cfg(feature = "cff")]
            cff: None,
            colr: None,
            cpal: None,
            #[cfg(feature = "layout")]
            gsub: None,
            hmtx_pos: None,
            loca_pos: None,
            glyf_pos: None,
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

    pub fn get_font_from_file(filename: &PathBuf) -> Option<Self> {
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
        let grif = if self.current_font == 0 {
            self.grif.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1]
                .grif
                .as_ref()
                .unwrap()
        };

        let glyph = grif.get_glyph(glyph_id).unwrap();
        let layout: HorizontalLayout = self.get_horizontal_layout(glyph_id);
        let open_type_glyph = OpenTypeGlyph {
            layout: FontLayout::Horizontal(layout),
            glyph: Box::new(glyph.clone()),
        };

        GriphData {
            glyph_id,
            format: GlyphFormat::OpenTypeGlyph,
            open_type_glif: Some(open_type_glyph),
        }
    }

    pub fn get_gryph(&self, ch: char) -> GriphData {
        let code = ch as u32;
        let (cmap, grif) = if self.current_font == 0 {
            (self.cmap.as_ref().unwrap(), self.grif.as_ref().unwrap())
        } else {
            (
                self.more_fonts[self.current_font - 1]
                    .cmap
                    .as_ref()
                    .unwrap(),
                self.more_fonts[self.current_font - 1]
                    .grif
                    .as_ref()
                    .unwrap(),
            )
        };

        let pos = cmap.get_griph_position(code);
        let glyph = grif.get_glyph(pos as usize).unwrap();
        let layout: HorizontalLayout = self.get_horizontal_layout(pos as usize);
        let open_type_glyph = OpenTypeGlyph {
            layout: FontLayout::Horizontal(layout),
            glyph: Box::new(glyph.clone()),
        };

        GriphData {
            glyph_id: pos as usize,
            format: GlyphFormat::OpenTypeGlyph,
            open_type_glif: Some(open_type_glyph),
        }
    }

    pub fn get_svg(&self, ch: char) -> String {
        // utf-32
        let glyf_data = self.get_gryph(ch);
        let pos = glyf_data.glyph_id;
        let glyf = glyf_data.open_type_glif.as_ref().unwrap().glyph.as_ref();
        let grif = if self.current_font == 0 {
            self.grif.as_ref().unwrap()
        } else {
            self.more_fonts[self.current_font - 1]
                .grif
                .as_ref()
                .unwrap()
        };
        let layout = &glyf_data.open_type_glif.as_ref().unwrap().layout;
        let layout = match layout {
            FontLayout::Horizontal(layout) => layout,
            _ => panic!("not support vertical layout"),
        };

        let fontsize = 24.0;
        let fontunit = "pt";

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
                return glyf.to_svg(fontsize, fontunit, &layout);
            }
            let mut string = glyf.get_svg_heder(fontsize, fontunit, &layout);
            #[cfg(debug_assertions)]
            {
                string += &format!("\n<!-- {} glyf id: {} -->", ch, pos);
            }

            for layer in layers {
                let glyf_id = layer.glyph_id as u32;
                let glyf = grif.get_glyph(glyf_id as usize).unwrap();
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
            string
        } else {
            #[cfg(debug_assertions)]
            {
                let string = glyf.to_svg(fontsize, fontunit, &layout);
                return format!("<!-- {} glyf id: {} -->{}", ch, pos, string);
            }
            #[cfg(not(debug_assertions))]
            glyf.to_svg(fontsize, fontunit, &layout)
        }
    }

    pub fn get_name(&self, name_id: NameID, locale: &String) -> String {
        let name_table = if self.current_font == 0 {
            self.name_table.as_ref().unwrap()
        } else {
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

    pub fn get_html(&self, string: &str) -> String {
        let mut html = String::new();
        html += "<html>\n";
        html += "<head>\n";
        html += "<meta charset=\"UTF-8\">\n";
        html += "<title>fontreader</title>\n";
        html += "</head>\n";
        html += "<body>\n";
        for ch in string.chars() {
            if ch == '\n' || ch == '\r' {
                html += "<br>\n";
                continue;
            }
            if ch == '\t' {
                html += "<span style=\"width: 4em; display: inline-block;\"></span>\n";
                continue;
            }
            let svg = self.get_svg(ch);
            html += &svg;
        }
        html += "</body>\n";
        html += "</html>\n";
        html
    }

    pub fn get_info(&self) -> String {
        let mut string = String::new();
        let name = self.name.as_ref().unwrap();
        let font_famiry = name.get_family_name();
        let subfamily_name = name.get_subfamily_name();
        string += &format!("Font famiry: {} {}\n", font_famiry, subfamily_name);
        for more_font in self.more_fonts.iter() {
            let name = more_font.name.as_ref().unwrap();
            let font_famiry = name.get_family_name();
            let subfamily_name = name.get_subfamily_name();
            string += &format!("Font famiry: {} {}\n", font_famiry, subfamily_name);
        }
        string
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

fn font_load_from_file(filename: &PathBuf) -> Option<Font> {
    let file = File::open(filename).unwrap();
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
    let glyf = _font.grif.as_ref().unwrap();
    for i in 0x0020..0x0ff {
        let pos = cmap_encodings.get_griph_position(i);
        let glyph = glyf.get_glyph(pos as usize).unwrap();
        let layout = _font.get_horizontal_layout(pos as usize);
        let svg = glyph.to_svg(32.0, "pt", &layout);
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
        let pos = cmap_encodings.get_griph_position(i as u32);
        let glyph = glyf.get_glyph(pos as usize).unwrap();
        let layout = _font.get_horizontal_layout(pos as usize);
        let svg = glyph.to_svg(100.0, "px", &layout);
        let ch = char::from_u32(i as u32).unwrap();
        write!(&mut writer, "{}:{:04} ", ch, pos).unwrap();
        writeln!(&mut writer, "{}", svg).unwrap();
    }
    writeln!(&mut writer).unwrap();
    let i = 0x2a6b2;
    let pos = cmap_encodings.get_griph_position(i as u32);
    let ch = char::from_u32(i as u32).unwrap();
    writeln!(&mut writer, "{}:{:04} ", ch, pos).unwrap();
}

fn font_load<R: BinaryReader>(file: &mut R) -> Option<Font> {
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
                match font.is_some() {
                    true => {
                        fonts.push(font.unwrap());
                    }
                    false => (),
                }
            }
            if let Some(font) = font.as_mut() {
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
            for table in woff.tables {
                let tag: [u8; 4] = [
                    (table.tag >> 24) as u8,
                    (table.tag >> 16) as u8,
                    (table.tag >> 8) as u8,
                    table.tag as u8,
                ];
                println!("tag: {}", crate::util::u32_to_string(table.tag));
                match &tag {
                    b"cmap" => {
                        let mut reader = BytesReader::new(&table.data);
                        let cmap_encodings =
                            CmapEncodings::new(&mut reader, 0, table.data.len() as u32);
                        font.cmap = Some(cmap_encodings);
                        println!("cmap");
                    }
                    b"head" => {
                        let mut reader = BytesReader::new(&table.data);
                        let head = head::HEAD::new(&mut reader, 0, table.data.len() as u32);
                        font.head = Some(head);
                    }
                    b"OS/2" => {
                        // let mut reader = BytesReader::new(&table.data);
                        // let os2 = os2::OS2::new(&mut reader, 0, table.data.len() as u32);
                        // font.os2 = Some(os2);
                    }
                    b"hhea" => {
                        let mut reader = BytesReader::new(&table.data);
                        let hhea = hhea::HHEA::new(&mut reader, 0, table.data.len() as u32);
                        font.hhea = Some(hhea);
                    }
                    b"maxp" => {
                        let mut reader = BytesReader::new(&table.data);
                        let maxp = maxp::MAXP::new(&mut reader, 0, table.data.len() as u32);
                        font.maxp = Some(maxp);
                    }
                    b"hmtx" => {
                        print!("hmtx");
                        print!("{} ", table.data.len());
                        hmtx_table = Some(table);
                    }
                    b"name" => {
                        let mut reader = BytesReader::new(&table.data);
                        let name = name::NAME::new(&mut reader, 0, table.data.len() as u32);
                        let name_table = name::NameTable::new(&name);
                        font.name = Some(name);
                        font.name_table = Some(name_table);
                    }
                    b"post" => {
                        let mut reader = BytesReader::new(&table.data);
                        let post = post::POST::new(&mut reader, 0, table.data.len() as u32);
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
                        let colr = colr::COLR::new(&mut reader, 0, table.data.len() as u32);
                        font.colr = Some(colr);
                    }
                    b"CPAL" => {
                        let mut reader = BytesReader::new(&table.data);
                        let cpal = cpal::CPAL::new(&mut reader, 0, table.data.len() as u32);
                        font.cpal = Some(cpal);
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
            );
            font.hmtx = Some(hmtx);
            let mut reader = BytesReader::new(&loca_table.as_ref().unwrap().data);
            let loca = loca::LOCA::new(
                &mut reader,
                0,
                loca_table.as_ref().unwrap().data.len() as u32,
                font.maxp.as_ref().unwrap().num_glyphs,
            );
            font.loca = Some(loca);
            let mut reader = BytesReader::new(&glyf_table.as_ref().unwrap().data);
            let glyf = glyf::GLYF::new(
                &mut reader,
                0,
                glyf_table.as_ref().unwrap().data.len() as u32,
                font.loca.as_ref().unwrap(),
            );
            font.grif = Some(glyf);
            #[cfg(debug_assertions)]
            {
                font_debug(&font);
            }
            Some(font)
        }
        fontheader::FontHeaders::WOFF2(_) => todo!(),
        fontheader::FontHeaders::Unknown => {
            //todo!(),
            None
        }
    }
}

fn from_opentype<R: BinaryReader>(file: &mut R, header: &OTFHeader) -> Option<Font> {
    let mut font = Font::empty();
    font.font_type = fontheader::FontHeaders::OTF(header.clone());

    header.table_records.as_ref().iter().for_each(|record| {
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
                let cmap_encodings = CmapEncodings::new(file, record.offset, record.length);
                font.cmap = Some(cmap_encodings);
            }
            b"head" => {
                let head = head::HEAD::new(file, record.offset, record.length);
                font.head = Some(head);
            }
            b"hhea" => {
                let hhea = hhea::HHEA::new(file, record.offset, record.length);
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
                let maxp = maxp::MAXP::new(file, record.offset, record.length);
                font.maxp = Some(maxp);
            }
            b"name" => {
                let name = name::NAME::new(file, record.offset, record.length);
                let name_table = name::NameTable::new(&name);
                font.name = Some(name);
                font.name_table = Some(name_table);
            }
            b"OS/2" => {
                let os2 = os2::OS2::new(file, record.offset, record.length);
                font.os2 = Some(os2);
            }
            b"post" => {
                let post = post::POST::new(file, record.offset, record.length);
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
                let colr = colr::COLR::new(file, record.offset, record.length);
                font.colr = Some(colr);
            }
            b"CPAL" => {
                let cpal = cpal::CPAL::new(file, record.offset, record.length);
                font.cpal = Some(cpal);
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
    });

    let num_glyphs = font.maxp.as_ref().unwrap().num_glyphs;
    let number_of_hmetrics = font.hhea.as_ref().unwrap().number_of_hmetrics;
    let offset = font.hmtx_pos.as_ref().unwrap().offset;
    let length = font.hmtx_pos.as_ref().unwrap().length;

    let hmtx = hmtx::HMTX::new(file, offset, length, number_of_hmetrics, num_glyphs);
    font.hmtx = Some(hmtx);

    if let Some(offset) = font.loca_pos.as_ref() {
        let length = font.loca_pos.as_ref().unwrap().length;
        let loca = loca::LOCA::new(file, offset.offset, length, num_glyphs);
        font.loca = Some(loca);
        let offset = font.glyf_pos.as_ref().unwrap().offset;
        let length = font.glyf_pos.as_ref().unwrap().length;
        let loca = font.loca.as_ref().unwrap();
        let glyf = glyf::GLYF::new(file, offset, length, loca);
        font.grif = Some(glyf);
    }

    if font.cmap.is_none() {
        debug_assert!(true, "No cmap table");
        return None;
    }
    if font.head.is_none() {
        debug_assert!(true, "No head table");
        return None;
    }
    if font.hhea.is_none() {
        debug_assert!(true, "No hhea table");
        return None;
    }
    if font.hmtx.is_none() {
        debug_assert!(true, "No hmtx table");
        return None;
    }
    if font.maxp.is_none() {
        debug_assert!(true, "No maxp table");
        return None;
    }
    if font.name.is_none() {
        debug_assert!(true, "No name table");
        return None;
    }

    /*
    if font.loca.is_none() {
        debug_assert!(true, "Not support no loca table, current only support OpenType font, not support CFF/CFF2/SVG font");
        return None;
    }
    if font.grif.is_none() {
        debug_assert!(true, "Not support no glyf table");
        return None;
    }
    */
    Some(font)
}
