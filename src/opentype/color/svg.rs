use miniz_oxide::inflate::decompress_to_vec_zlib;
use std::io::SeekFrom;

use bin_rs::reader::BinaryReader;

use crate::fontreader::FontLayout;

#[derive(Debug, Clone)]
pub(crate) struct SVG {
    version: u16,
    svg_document_list: SVGDocumetList,
    // reserved: u32,
}

impl SVG {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        _: u32,
    ) -> Result<Self, std::io::Error> {
        let offset = offset as u64;
        reader.seek(SeekFrom::Start(offset))?;
        let version = reader.read_u16_be()?;
        let svg_document_list_offset = reader.read_u32_be()?;
        reader.read_u32()?; // reserved

        let svg_document_list =
            SVGDocumetList::new(reader, offset + svg_document_list_offset as u64)?;

        Ok(Self {
            version,
            svg_document_list,
        })
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = "SVG\n".to_string();
        string += &format!("version: {}\n", self.version);
        string += &format!(
            "svg_document_list: {}\n",
            self.svg_document_list.svg_document_records.len()
        );
        for (i, svg_document_record) in self
            .svg_document_list
            .svg_document_records
            .iter()
            .enumerate()
        {
            if i > 10 {
                break;
            }
            string += &format!("svg_document_record {}\n", i);
            string += &format!("  start_glyph_id: {}\n", svg_document_record.start_glyph_id);
            string += &format!("  end_glyph_id: {}\n", svg_document_record.end_glyph_id);
        }
        string
    }

    pub(crate) fn get_svg(
        &self,
        gid: u32,
        fonsize: f64,
        fontunit: &str,
        layout: &FontLayout,
        _: f64,
        _: f64,
    ) -> Option<String> {
        let gid = gid as u16;
        let mut svg_document = None;
        for svg_document_record in self.svg_document_list.svg_document_records.iter() {
            if svg_document_record.start_glyph_id <= gid && gid <= svg_document_record.end_glyph_id
            {
                svg_document = svg_document_record.svg_document.as_ref();
                break;
            }
        }
        if svg_document.is_none() {
            return None;
        }
        let wrapped_svg = svg_document.unwrap();
        let svg = if wrapped_svg[0] == 0x1f && wrapped_svg[1] == 0x8b && wrapped_svg[2] == 0x08 {
            let decompress = decompress_to_vec_zlib(&wrapped_svg).unwrap();
            String::from_utf8(decompress.to_vec()).unwrap()
        } else {
            String::from_utf8(wrapped_svg.to_vec()).unwrap()
        };

        // 頭の "<svg" を探す
        let svgs = svg.split("<svg").collect::<Vec<&str>>();
        if svgs.len() < 2 {
            return None;
        }
        let mut string = format!(
            "<svg width=\"{}{}\" height=\"{}{}\" ",
            fonsize, fontunit, fonsize, fontunit
        );
        match layout {
            FontLayout::Horizontal(layout) => {
                string += &format!(
                    " viewbox=\"{} {} {} {}\"",
                    0,
                    -layout.accender,
                    layout.advance_width,
                    layout.accender - layout.descender + layout.line_gap
                );
            }
            FontLayout::Vertical(layout) => {
                string += &format!(
                    " viewbox=\"{} {} {} {}\"",
                    0,
                    -layout.accender,
                    layout.advance_height,
                    layout.accender - layout.descender + layout.line_gap,
                );
            }
            _ => {}
        }
        for i in 1..svgs.len() {
            string += &svgs[i];
        }
        Some(string)
    }
}

#[derive(Debug, Clone)]
struct SVGDocumetList {
    svg_document_records: Vec<SVGDocumetRecord>,
}

impl SVGDocumetList {
    fn new<R: BinaryReader>(reader: &mut R, offset: u64) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let num_svg_entries = reader.read_u16_be()?;
        let mut svg_document_records = Vec::new();
        for _ in 0..num_svg_entries {
            svg_document_records.push(SVGDocumetRecord::new(reader)?);
        }
        for svg_document_record in svg_document_records.iter_mut() {
            reader.seek(SeekFrom::Start(
                offset + svg_document_record.svg_document_offset as u64,
            ))?;
            let svg_document =
                reader.read_bytes_as_vec(svg_document_record.svg_document_length as usize)?;
            svg_document_record.svg_document = Some(svg_document);
        }

        Ok(Self {
            svg_document_records,
        })
    }
}

#[derive(Debug, Clone)]
struct SVGDocumetRecord {
    start_glyph_id: u16,
    end_glyph_id: u16,
    svg_document_offset: u32,
    svg_document_length: u32,
    svg_document: Option<Vec<u8>>,
}

impl SVGDocumetRecord {
    fn new<R: BinaryReader>(reader: &mut R) -> Result<Self, std::io::Error> {
        let start_glyph_id = reader.read_u16_be()?;
        let end_glyph_id = reader.read_u16_be()?;
        let svg_document_offset = reader.read_u32_be()?;
        let svg_document_length = reader.read_u32_be()?;

        Ok(Self {
            start_glyph_id,
            end_glyph_id,
            svg_document_offset,
            svg_document_length,
            svg_document: None,
        })
    }
}
