#![allow(dead_code)]

use miniz_oxide::inflate::decompress_to_vec;
use std::io::SeekFrom;

use bin_rs::reader::BinaryReader;

use crate::fontreader::FontLayout;

#[derive(Debug, Clone)]
pub(crate) struct SvgGlyphDocument {
    pub(crate) payload: String,
    pub(crate) view_box_min_x: f32,
    pub(crate) view_box_min_y: f32,
    pub(crate) view_box_width: f32,
    pub(crate) view_box_height: f32,
}

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

    pub(crate) fn has_glyph(&self, gid: u32) -> bool {
        let gid = gid as u16;
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
        _: f64,
        _: f64,
    ) -> Option<String> {
        let document = self.get_glyph_document(gid, layout)?;
        let mut string = format!(
            "<svg width=\"{}{}\" height=\"{}{}\" viewBox=\"{} {} {} {}\">",
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

    pub(crate) fn get_glyph_document(
        &self,
        gid: u32,
        layout: &FontLayout,
    ) -> Option<SvgGlyphDocument> {
        let gid = gid as u16;
        let mut svg_document = None;
        for svg_document_record in self.svg_document_list.svg_document_records.iter() {
            if svg_document_record.start_glyph_id <= gid && gid <= svg_document_record.end_glyph_id
            {
                svg_document = svg_document_record.svg_document.as_ref();
                break;
            }
        }
        let svg = decode_svg_document(svg_document?)?;
        let payload = extract_svg_payload_for_gid(&svg, gid)?;
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

fn decode_svg_document(document: &[u8]) -> Option<String> {
    if document.len() >= 3 && document[0] == 0x1f && document[1] == 0x8b && document[2] == 0x08 {
        let decompress = decompress_gzip(document)?;
        String::from_utf8(decompress).ok()
    } else {
        String::from_utf8(document.to_vec()).ok()
    }
}

fn decompress_gzip(document: &[u8]) -> Option<Vec<u8>> {
    if document.len() < 18 {
        return None;
    }
    let flags = document[3];
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

    decompress_to_vec(&document[cursor..document.len() - 8]).ok()
}

fn extract_svg_payload_for_gid(document: &str, gid: u16) -> Option<String> {
    let (root_inner, defs_blocks) = if let Some(inner) = extract_root_inner(document) {
        (inner, collect_tag_blocks(inner, "defs"))
    } else {
        ("", Vec::new())
    };

    if let Some(payload) = extract_payload_from_svg_fragments(root_inner, gid, &defs_blocks) {
        return Some(payload);
    }

    if let Some(payload) = extract_payload_from_tagged_elements(root_inner, gid, &defs_blocks) {
        return Some(payload);
    }

    if let Some(inner) = extract_root_inner(document) {
        return Some(inner.to_string());
    }

    extract_payload_from_svg_fragments(document, gid, &[])
        .or_else(|| extract_payload_from_tagged_elements(document, gid, &[]))
}

fn extract_payload_from_svg_fragments(
    source: &str,
    gid: u16,
    defs_blocks: &[&str],
) -> Option<String> {
    let fragments = collect_tag_blocks(source, "svg");
    if fragments.is_empty() {
        return None;
    }

    let selected = fragments
        .iter()
        .find(|fragment| fragment_matches_glyph_id(fragment, gid))
        .copied()
        .or_else(|| {
            if fragments.len() == 1 {
                fragments.first().copied()
            } else {
                None
            }
        })?;

    let fragment_inner = extract_root_inner(selected).unwrap_or(selected);
    Some(combine_defs_and_payload(defs_blocks, fragment_inner))
}

fn extract_payload_from_tagged_elements(
    source: &str,
    gid: u16,
    defs_blocks: &[&str],
) -> Option<String> {
    let elements = collect_glyph_tagged_elements(source, gid);
    if elements.is_empty() {
        return None;
    }

    let mut payload = String::new();
    for defs in defs_blocks {
        payload += defs;
    }
    for element in elements {
        payload += element;
    }
    Some(payload)
}

fn combine_defs_and_payload(defs_blocks: &[&str], payload: &str) -> String {
    let mut combined = String::new();
    for defs in defs_blocks {
        combined += defs;
    }
    combined += payload;
    combined
}

fn extract_root_inner(document: &str) -> Option<&str> {
    let root_start = document.find("<svg")?;
    let start_tag_end = document[root_start..].find('>')? + root_start;
    let root_end = document.rfind("</svg>")?;
    if root_end <= start_tag_end {
        return None;
    }
    Some(&document[start_tag_end + 1..root_end])
}

fn collect_tag_blocks<'a>(source: &'a str, tag: &str) -> Vec<&'a str> {
    let mut blocks = Vec::new();
    let open_pattern = format!("<{}", tag);
    let mut cursor = 0usize;

    while let Some(relative_start) = source[cursor..].find(&open_pattern) {
        let start = cursor + relative_start;
        let Some(end) = find_balanced_tag_end(source, start, tag) else {
            break;
        };
        blocks.push(&source[start..end]);
        cursor = end;
    }

    blocks
}

fn find_balanced_tag_end(source: &str, start: usize, tag: &str) -> Option<usize> {
    let open_end = source[start..].find('>')? + start;
    if source[..=open_end].ends_with("/>") {
        return Some(open_end + 1);
    }

    let open_pattern = format!("<{}", tag);
    let close_pattern = format!("</{}", tag);
    let mut depth = 1usize;
    let mut cursor = open_end + 1;

    while cursor < source.len() {
        let next_open = source[cursor..]
            .find(&open_pattern)
            .map(|offset| cursor + offset);
        let next_close = source[cursor..]
            .find(&close_pattern)
            .map(|offset| cursor + offset);

        match (next_open, next_close) {
            (None, Some(close_start)) | (Some(_), Some(close_start))
                if next_open
                    .map(|open_start| close_start < open_start)
                    .unwrap_or(true) =>
            {
                let close_end = source[close_start..].find('>')? + close_start + 1;
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(close_end);
                }
                cursor = close_end;
            }
            (Some(open_start), _) => {
                let nested_open_end = source[open_start..].find('>')? + open_start;
                if !source[..=nested_open_end].ends_with("/>") {
                    depth += 1;
                }
                cursor = nested_open_end + 1;
            }
            (None, Some(close_start)) => {
                let close_end = source[close_start..].find('>')? + close_start + 1;
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(close_end);
                }
                cursor = close_end;
            }
            (None, None) => break,
        }
    }

    None
}

fn fragment_matches_glyph_id(fragment: &str, gid: u16) -> bool {
    let lower = fragment.to_ascii_lowercase();
    let marker = format!("glyph{}", gid);
    let gid_marker = format!("id=\"{}\"", gid);
    lower.contains(&marker) || lower.contains(&gid_marker)
}

fn collect_glyph_tagged_elements<'a>(source: &'a str, gid: u16) -> Vec<&'a str> {
    let lower = source.to_ascii_lowercase();
    let markers = [format!("glyph{}", gid), format!("id=\"{}\"", gid)];
    let mut elements = Vec::new();

    for marker in markers {
        let mut cursor = 0usize;
        while let Some(relative_pos) = lower[cursor..].find(&marker) {
            let pos = cursor + relative_pos;
            let Some(start) = source[..pos].rfind('<') else {
                cursor = pos + marker.len();
                continue;
            };
            if source[start..].starts_with("</") {
                cursor = pos + marker.len();
                continue;
            }

            let tag_name_end = source[start + 1..]
                .find(|ch: char| ch.is_whitespace() || ch == '>' || ch == '/')
                .map(|offset| start + 1 + offset);
            let Some(tag_name_end) = tag_name_end else {
                cursor = pos + marker.len();
                continue;
            };
            let tag = &source[start + 1..tag_name_end];
            let Some(end) = find_balanced_tag_end(source, start, tag) else {
                cursor = pos + marker.len();
                continue;
            };
            let candidate = &source[start..end];
            if !elements.iter().any(|existing| *existing == candidate) {
                elements.push(candidate);
            }
            cursor = end;
        }
    }

    elements
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

#[cfg(test)]
mod tests {
    use super::*;

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

        assert!(payload.contains("linearGradient"));
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

        assert!(payload.contains("clipPath"));
        assert!(payload.contains("glyph4-shape"));
        assert!(!payload.contains("glyph3-shape"));
    }
}
