#![allow(dead_code)]

use miniz_oxide::inflate::decompress_to_vec_with_limit;
use std::io::{Error, ErrorKind, SeekFrom};
use std::sync::Arc;

use bin_rs::reader::BinaryReader;

use crate::fontreader::FontLayout;
use crate::limits::DecodeLimits;

#[derive(Debug, Clone)]
pub(crate) struct SvgGlyphDocument {
    pub(crate) payload: String,
    pub(crate) view_box_min_x: f32,
    pub(crate) view_box_min_y: f32,
    pub(crate) view_box_width: f32,
    pub(crate) view_box_height: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct Svg {
    version: u16,
    svg_document_list: SVGDocumetList,
    max_svg_bytes: usize,
    // reserved: u32,
}

impl Svg {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        length: u32,
    ) -> Result<Self, std::io::Error> {
        Self::new_with_limits(reader, offset, length, &DecodeLimits::default())
    }

    pub(crate) fn new_with_limits<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        length: u32,
        limits: &DecodeLimits,
    ) -> Result<Self, std::io::Error> {
        if length < 10 {
            return Err(Error::new(ErrorKind::InvalidData, "short SVG header"));
        }
        let offset = offset as u64;
        reader.seek(SeekFrom::Start(offset))?;
        let version = reader.read_u16_be()?;
        let svg_document_list_offset = reader.read_u32_be()?;
        reader.read_u32()?; // reserved

        if version != 0 || svg_document_list_offset < 10 || svg_document_list_offset > length - 2 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "invalid SVG document list",
            ));
        }
        let svg_document_list = SVGDocumetList::new(
            reader,
            offset + svg_document_list_offset as u64,
            (length - svg_document_list_offset) as usize,
        )?;

        Ok(Self {
            version,
            svg_document_list,
            max_svg_bytes: limits.max_svg_bytes,
        })
    }

    fn dump_text(&self) -> String {
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

    pub(crate) fn has_glyph(&self, gid: u32) -> bool {
        let Ok(gid) = u16::try_from(gid) else {
            return false;
        };
        self.svg_document_list
            .svg_document_records
            .iter()
            .any(|record| record.start_glyph_id <= gid && gid <= record.end_glyph_id)
    }

    pub(crate) fn get_svg(
        &self,
        gid: u32,
        fonsize: f64,
        fontunit: &str,
        layout: &FontLayout,
        units_per_em: u16,
    ) -> Option<String> {
        let document = self.get_glyph_document_with_units(gid, layout, units_per_em)?;
        let mut string = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}{}\" height=\"{}{}\" viewBox=\"{} {} {} {}\">",
            fonsize,
            fontunit,
            fonsize,
            fontunit,
            document.view_box_min_x,
            document.view_box_min_y,
            document.view_box_width,
            document.view_box_height
        );
        string += &document.payload;
        string += "</svg>";
        Some(string)
    }

    #[cfg(test)]
    pub(crate) fn get_glyph_document(
        &self,
        gid: u32,
        layout: &FontLayout,
    ) -> Option<SvgGlyphDocument> {
        self.get_glyph_document_with_units(gid, layout, 1000)
    }

    pub(crate) fn get_glyph_document_with_units(
        &self,
        gid: u32,
        layout: &FontLayout,
        units_per_em: u16,
    ) -> Option<SvgGlyphDocument> {
        let gid = u16::try_from(gid).ok()?;
        let mut svg_document = None;
        for svg_document_record in self.svg_document_list.svg_document_records.iter() {
            if svg_document_record.start_glyph_id <= gid && gid <= svg_document_record.end_glyph_id
            {
                svg_document = svg_document_record.svg_document.as_deref();
                break;
            }
        }
        let svg = decode_svg_document(svg_document?, self.max_svg_bytes)?;
        let payload = super::svg_extract::extract_with_units(&svg, gid, units_per_em)?;
        let (view_box_min_x, view_box_min_y, view_box_width, view_box_height) =
            layout_view_box(layout);
        Some(SvgGlyphDocument {
            payload,
            view_box_min_x,
            view_box_min_y,
            view_box_width,
            view_box_height,
        })
    }
}

fn layout_view_box(layout: &FontLayout) -> (f32, f32, f32, f32) {
    match layout {
        FontLayout::Horizontal(layout) => (
            0.0,
            -layout.accender as f32,
            layout.advance_width as f32,
            (layout.accender - layout.descender + layout.line_gap) as f32,
        ),
        FontLayout::Vertical(layout) => (
            0.0,
            -layout.accender as f32,
            layout.advance_height as f32,
            (layout.accender - layout.descender + layout.line_gap) as f32,
        ),
        FontLayout::Unknown => (0.0, 0.0, 0.0, 0.0),
    }
}

fn decode_svg_document(document: &[u8], max_svg_bytes: usize) -> Option<String> {
    if document.len() >= 3 && document[0] == 0x1f && document[1] == 0x8b && document[2] == 0x08 {
        let decompress = decompress_gzip(document, max_svg_bytes)?;
        String::from_utf8(decompress).ok()
    } else {
        if document.len() > max_svg_bytes {
            return None;
        }
        String::from_utf8(document.to_vec()).ok()
    }
}

fn decompress_gzip(document: &[u8], max_output: usize) -> Option<Vec<u8>> {
    if document.len() < 18 {
        return None;
    }
    let flags = document[3];
    if flags & 0xe0 != 0 {
        return None;
    }
    let mut cursor = 10usize;

    if flags & 0x04 != 0 {
        let extra_len = u16::from_le_bytes([*document.get(cursor)?, *document.get(cursor + 1)?]);
        cursor += 2 + extra_len as usize;
    }
    if flags & 0x08 != 0 {
        while *document.get(cursor)? != 0 {
            cursor += 1;
        }
        cursor += 1;
    }
    if flags & 0x10 != 0 {
        while *document.get(cursor)? != 0 {
            cursor += 1;
        }
        cursor += 1;
    }
    if flags & 0x02 != 0 {
        cursor += 2;
    }

    if cursor >= document.len().saturating_sub(8) {
        return None;
    }

    let expected_crc = u32::from_le_bytes([
        *document.get(document.len() - 8)?,
        *document.get(document.len() - 7)?,
        *document.get(document.len() - 6)?,
        *document.get(document.len() - 5)?,
    ]);
    let expected_size = u32::from_le_bytes([
        *document.get(document.len() - 4)?,
        *document.get(document.len() - 3)?,
        *document.get(document.len() - 2)?,
        *document.last()?,
    ]) as usize;
    if expected_size > max_output {
        return None;
    }
    let decompressed =
        decompress_to_vec_with_limit(&document[cursor..document.len() - 8], max_output).ok()?;
    if decompressed.len() != expected_size {
        return None;
    }
    if crc32(&decompressed) != expected_crc {
        return None;
    }
    Some(decompressed)
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

pub(super) fn find_svg_tag_end(payload: &str, start: usize) -> Option<usize> {
    let bytes = payload.as_bytes();
    let mut quote = None;
    for (index, &byte) in bytes.iter().enumerate().skip(start + 1) {
        match (quote, byte) {
            (Some(expected), byte) if byte == expected => quote = None,
            (None, b'\'' | b'\"') => quote = Some(byte),
            (None, b'>') => return Some(index + 1),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
fn extract_svg_payload_for_gid(document: &str, gid: u16) -> Option<String> {
    super::svg_extract::extract(document, gid)
}

#[derive(Debug, Clone)]
struct SVGDocumetList {
    svg_document_records: Vec<SVGDocumetRecord>,
}

impl SVGDocumetList {
    fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        length: usize,
    ) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let num_svg_entries = reader.read_u16_be()?;
        let records_end = 2 + num_svg_entries as usize * 12;
        if records_end > length {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "SVG records exceed table",
            ));
        }
        let mut svg_document_records = Vec::new();
        for _ in 0..num_svg_entries {
            svg_document_records.push(SVGDocumetRecord::new(reader)?);
        }
        let mut documents = std::collections::HashMap::new();
        let mut previous_end = None;
        for svg_document_record in svg_document_records.iter_mut() {
            let record = &svg_document_record;
            let start = record.svg_document_offset as usize;
            let size = record.svg_document_length as usize;
            if record.start_glyph_id > record.end_glyph_id
                || previous_end.is_some_and(|end| record.start_glyph_id <= end)
                || start < records_end
                || start > length
                || size > length - start
            {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "invalid SVG document range",
                ));
            }
            previous_end = Some(record.end_glyph_id);
            let key = (record.svg_document_offset, record.svg_document_length);
            if let Some(document) = documents.get(&key) {
                svg_document_record.svg_document = Some(Arc::clone(document));
                continue;
            }
            reader.seek(SeekFrom::Start(
                offset + svg_document_record.svg_document_offset as u64,
            ))?;
            let svg_document =
                reader.read_bytes_as_vec(svg_document_record.svg_document_length as usize)?;
            let svg_document: Arc<[u8]> = svg_document.into();
            documents.insert(key, Arc::clone(&svg_document));
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
    svg_document: Option<Arc<[u8]>>,
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

impl std::fmt::Display for Svg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Opt-in real OpenType-SVG corpus check, no network access in tests.
    #[test]
    #[ignore = "requires FONTCORE_SVG_FONT pointing at an OpenType-SVG font"]
    fn external_svg_font_corpus() {
        use bin_rs::reader::BytesReader;
        let path = std::env::var("FONTCORE_SVG_FONT").unwrap();
        let bytes = std::fs::read(path).unwrap();
        let count = u16::from_be_bytes(bytes[4..6].try_into().unwrap()) as usize;
        let mut svg_table = None;
        let mut units = None;
        for index in 0..count {
            let start = 12 + index * 16;
            let offset = u32::from_be_bytes(bytes[start + 8..start + 12].try_into().unwrap());
            let length = u32::from_be_bytes(bytes[start + 12..start + 16].try_into().unwrap());
            match &bytes[start..start + 4] {
                b"SVG " => svg_table = Some((offset, length)),
                b"head" => {
                    units = Some(u16::from_be_bytes(
                        bytes[offset as usize + 18..offset as usize + 20]
                            .try_into()
                            .unwrap(),
                    ))
                }
                _ => {}
            }
        }
        let (offset, length) = svg_table.unwrap();
        let svg = Svg::new(&mut BytesReader::new(&bytes), offset, length).unwrap();
        let mut accepted = 0usize;
        let mut rejected = Vec::new();
        for record in &svg.svg_document_list.svg_document_records {
            for gid in record.start_glyph_id..=record.end_glyph_id {
                if svg
                    .get_glyph_document_with_units(gid as u32, &FontLayout::Unknown, units.unwrap())
                    .is_some()
                {
                    accepted += 1;
                } else {
                    rejected.push(gid);
                }
            }
        }
        eprintln!(
            "SVG corpus: {accepted} accepted, {} rejected: {:?}",
            rejected.len(),
            rejected
        );
        assert!(accepted > 0);
        assert!(rejected.is_empty());
    }

    fn table(document: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0; 18];
        bytes[2..6].copy_from_slice(&18u32.to_be_bytes());
        bytes.extend_from_slice(&2u16.to_be_bytes());
        for gid in [1u16, 10] {
            bytes.extend_from_slice(&gid.to_be_bytes());
            bytes.extend_from_slice(&gid.to_be_bytes());
            bytes.extend_from_slice(&26u32.to_be_bytes());
            bytes.extend_from_slice(&(document.len() as u32).to_be_bytes());
        }
        bytes.extend_from_slice(document);
        bytes
    }

    #[test]
    fn untagged_svg_wrappers_cannot_bypass_glyph_selection() {
        let svg = "<svg><svg><g id='glyph1'><path d='M1 0'/></g><g id='glyph10'><path d='M10 0'/></g></svg></svg>";
        assert!(extract_svg_payload_for_gid(svg, 2).is_none());
        let selected = extract_svg_payload_for_gid(svg, 1).unwrap();
        assert!(selected.contains("glyph1"));
        assert!(!selected.contains("glyph10"));
        let wrapped = format!("<svg>{svg}</svg>");
        assert!(extract_svg_payload_for_gid(&wrapped, 2).is_none());
        assert!(!extract_svg_payload_for_gid(&wrapped, 1)
            .unwrap()
            .contains("glyph10"));
        let plain = "<svg><svg><path d='M1 0'/></svg></svg>";
        assert!(extract_svg_payload_for_gid(plain, 1).is_some());
    }

    #[test]
    fn document_root_glyph_id_is_not_discarded() {
        let svg = "<svg id='glyph10'><path d='M10 0'/></svg>";
        assert!(extract_svg_payload_for_gid(svg, 1).is_none());
        assert!(extract_svg_payload_for_gid(svg, 10).is_some());
    }

    #[test]
    fn table_offsets_shared_documents_and_bounds() {
        use bin_rs::reader::BytesReader;
        let bytes = table(b"<svg><g id='glyph1'/></svg>");
        let svg = Svg::new(&mut BytesReader::new(&bytes), 0, bytes.len() as u32).unwrap();
        let records = &svg.svg_document_list.svg_document_records;
        assert!(Arc::ptr_eq(
            records[0].svg_document.as_ref().unwrap(),
            records[1].svg_document.as_ref().unwrap()
        ));
        let mut embedded = vec![0xff; 7];
        embedded.extend_from_slice(&bytes);
        assert!(Svg::new(&mut BytesReader::new(&embedded), 7, bytes.len() as u32).is_ok());
        let mut bad = bytes.clone();
        bad[24..28].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(Svg::new(&mut BytesReader::new(&bad), 0, bad.len() as u32).is_err());
        assert!(svg.has_glyph(1));
        assert!(!svg.has_glyph(65537));
        assert!(Svg::new(&mut BytesReader::new(&bytes), 0, bytes.len() as u32 - 1).is_err());
        assert!(Svg::new(&mut BytesReader::new(&bytes), 0, 9).is_err());
    }

    #[test]
    fn compressed_size_is_not_the_decoded_size_limit() {
        use bin_rs::reader::BytesReader;
        let plain = format!(
            "<svg><g id='glyph1'>{}</g></svg>",
            "<path d='M0 0'/>".repeat(80)
        );
        let mut gzip = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0];
        gzip.extend(miniz_oxide::deflate::compress_to_vec(plain.as_bytes(), 6));
        gzip.extend(crc32(plain.as_bytes()).to_le_bytes());
        gzip.extend((plain.len() as u32).to_le_bytes());
        let bytes = table(&gzip);
        assert!(bytes.len() < plain.len());
        let svg = Svg::new(&mut BytesReader::new(&bytes), 0, bytes.len() as u32).unwrap();
        assert!(svg.get_glyph_document(1, &FontLayout::Unknown).is_some());
        assert!(decode_svg_document(&gzip, plain.len() - 1).is_none());
    }

    #[test]
    fn glyph_ids_match_exact_attributes_not_substrings() {
        let svg =
            "<svg><g id='glyph10'><path d='M10 0'/></g><g id = 'glyph1'><path d='M1 0'/></g></svg>";
        let payload = extract_svg_payload_for_gid(svg, 1).unwrap();
        assert!(extract_svg_payload_for_gid(svg, 2).is_none());
        assert!(!payload.contains("glyph10"));
        assert!(payload.contains("glyph1"));
    }

    #[test]
    fn svg_payload_extracts_only_matching_nested_fragment() {
        let document = concat!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\">",
            "<defs><linearGradient id=\"grad\"/></defs>",
            "<svg id=\"glyph1\"><path id=\"glyph1-path\" d=\"M0 0\"/></svg>",
            "<svg id=\"glyph2\"><path id=\"glyph2-path\" d=\"M1 1\"/></svg>",
            "</svg>"
        );

        let payload = extract_svg_payload_for_gid(document, 2).expect("svg payload");

        assert!(!payload.contains("linearGradient"));
        assert!(payload.contains("glyph2-path"));
        assert!(!payload.contains("glyph1-path"));
    }

    #[test]
    fn svg_payload_extracts_matching_group_from_shared_document() {
        let document = concat!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\">",
            "<defs><clipPath id=\"clip\"/></defs>",
            "<g id=\"glyph3\"><path id=\"glyph3-shape\" d=\"M0 0\"/></g>",
            "<g id=\"glyph4\"><path id=\"glyph4-shape\" d=\"M1 1\"/></g>",
            "</svg>"
        );

        let payload = extract_svg_payload_for_gid(document, 4).expect("svg payload");

        assert!(!payload.contains("clipPath"));
        assert!(payload.contains("glyph4-shape"));
        assert!(!payload.contains("glyph3-shape"));
    }

    fn validated_fragment(fragment: &str) -> Option<String> {
        extract_svg_payload_for_gid(&format!("<svg>{fragment}</svg>"), 1)
    }

    #[test]
    fn svg_payload_rejects_active_content_and_external_references() {
        assert!(validated_fragment("<path d=\"M0 0\"/>").is_some());
        assert!(validated_fragment("<script>alert(1)</script>").is_none());
        assert!(validated_fragment("<path onclick=\"alert(1)\"/>").is_none());
        assert!(validated_fragment("<use href=\"https://example.test/x\"/>").is_none());
        assert!(validated_fragment("<path\nonload = \"alert(1)\"/>").is_none());
        assert!(validated_fragment("<use href = \" javascript:alert(1)\"/>").is_none());
        assert!(validated_fragment("<style>@import url(https://example.test);</style>").is_none());
    }

    #[test]
    fn svg_document_respects_caller_decode_limit() {
        assert!(decode_svg_document(b"<svg/>", 5).is_none());
        assert!(decode_svg_document(b"<svg/>", 6).is_some());
    }
}
