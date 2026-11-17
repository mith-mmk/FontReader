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
        let payload = extract_svg_payload_for_gid(&svg, gid)?;
        let payload = sanitize_svg_payload(&payload)?;
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

fn sanitize_svg_payload(payload: &str) -> Option<String> {
    let mut cursor = 0usize;
    while let Some(relative) = payload.get(cursor..)?.find('<') {
        let start = cursor + relative;
        let end = find_svg_tag_end(payload, start)?;
        validate_svg_tag(&payload[start + 1..end])?;
        cursor = end;
    }
    if payload.get(cursor..)?.contains('>') {
        return None;
    }
    Some(payload.to_string())
}

fn find_svg_tag_end(payload: &str, start: usize) -> Option<usize> {
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

fn validate_svg_tag(tag: &str) -> Option<()> {
    const SAFE_ELEMENTS: &[&str] = &[
        "svg",
        "g",
        "path",
        "rect",
        "circle",
        "ellipse",
        "line",
        "polyline",
        "polygon",
        "defs",
        "lineargradient",
        "radialgradient",
        "stop",
        "clippath",
        "mask",
        "use",
    ];
    let bytes = tag.as_bytes();
    let mut cursor = 0usize;
    skip_svg_whitespace(bytes, &mut cursor);
    if bytes.get(cursor).copied() == Some(b'!') || bytes.get(cursor).copied() == Some(b'?') {
        return None;
    }
    let closing = bytes.get(cursor).copied() == Some(b'/');
    if closing {
        cursor += 1;
        skip_svg_whitespace(bytes, &mut cursor);
    }
    let name_start = cursor;
    while bytes
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b':')
    {
        cursor += 1;
    }
    if name_start == cursor {
        return None;
    }
    let name = tag.get(name_start..cursor)?.to_ascii_lowercase();
    if !SAFE_ELEMENTS.contains(&name.as_str()) {
        return None;
    }
    if closing {
        skip_svg_whitespace(bytes, &mut cursor);
        return (cursor == bytes.len() - 1).then_some(());
    }

    loop {
        skip_svg_whitespace(bytes, &mut cursor);
        if cursor >= bytes.len() - 1 {
            return Some(());
        }
        if bytes[cursor] == b'/' {
            cursor += 1;
            skip_svg_whitespace(bytes, &mut cursor);
            return (cursor == bytes.len() - 1).then_some(());
        }

        let attr_start = cursor;
        while bytes
            .get(cursor)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && !matches!(*byte, b'=' | b'>'))
        {
            cursor += 1;
        }
        if attr_start == cursor {
            return None;
        }
        let attribute = tag.get(attr_start..cursor)?.to_ascii_lowercase();
        if attribute.starts_with("on")
            || matches!(attribute.as_str(), "style" | "src" | "action" | "begin")
        {
            return None;
        }
        skip_svg_whitespace(bytes, &mut cursor);
        if bytes.get(cursor).copied() != Some(b'=') {
            return None;
        }
        cursor += 1;
        skip_svg_whitespace(bytes, &mut cursor);
        let quote = *bytes.get(cursor)?;
        if quote != b'\'' && quote != b'\"' {
            return None;
        }
        cursor += 1;
        let value_start = cursor;
        while bytes.get(cursor).copied() != Some(quote) {
            cursor += 1;
            if cursor >= bytes.len() {
                return None;
            }
        }
        let value = tag.get(value_start..cursor)?;
        let compact = value
            .chars()
            .filter(|character| !character.is_ascii_whitespace())
            .collect::<String>()
            .to_ascii_lowercase();
        if compact.contains("javascript:")
            || compact.contains("vbscript:")
            || compact.contains("data:")
            || compact.contains("file:")
            || compact.contains("http:")
            || compact.contains("https:")
            || compact.contains("//")
        {
            return None;
        }
        if (attribute == "href" || attribute == "xlink:href") && !compact.starts_with("#") {
            return None;
        }
        let mut url_cursor = 0usize;
        while let Some(relative) = compact.get(url_cursor..)?.find("url(") {
            let url_start = url_cursor + relative + 4;
            if !compact.get(url_start..)?.starts_with('#') {
                return None;
            }
            url_cursor = url_start;
        }
        cursor += 1;
    }
}

fn skip_svg_whitespace(bytes: &[u8], cursor: &mut usize) {
    while bytes
        .get(*cursor)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        *cursor += 1;
    }
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

    // A shared document with glyph IDs must not render every sibling when the
    // requested glyph is absent. Keep the legacy untagged single-document path.
    let mut cursor = 0;
    while let Some(relative) = root_inner[cursor..].find('<') {
        let start = cursor + relative;
        let end = find_svg_tag_end(root_inner, start)?;
        if element_id(&root_inner[start..end]).is_some_and(|id| {
            id.strip_prefix("glyph")
                .is_some_and(|suffix| suffix.parse::<u16>().is_ok())
                || id.parse::<u16>().is_ok()
        }) {
            return None;
        }
        cursor = end;
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
            if fragments.len() == 1 && element_id(fragments[0]).is_none() {
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

// Match the ID attribute on this element only, never a descendant or substring.
fn element_id(fragment: &str) -> Option<&str> {
    let end = find_svg_tag_end(fragment, 0)?;
    let tag = &fragment[1..end - 1];
    let bytes = tag.as_bytes();
    let mut cursor = 0;
    while bytes.get(cursor).is_some_and(|b| !b.is_ascii_whitespace()) {
        cursor += 1;
    }
    loop {
        skip_svg_whitespace(bytes, &mut cursor);
        let start = cursor;
        while bytes
            .get(cursor)
            .is_some_and(|b| !b.is_ascii_whitespace() && *b != b'=')
        {
            cursor += 1;
        }
        let name = tag.get(start..cursor)?;
        skip_svg_whitespace(bytes, &mut cursor);
        if bytes.get(cursor) != Some(&b'=') {
            return None;
        }
        cursor += 1;
        skip_svg_whitespace(bytes, &mut cursor);
        let quote = *bytes.get(cursor)?;
        if quote != b'\'' && quote != b'"' {
            return None;
        }
        cursor += 1;
        let start = cursor;
        while *bytes.get(cursor)? != quote {
            cursor += 1;
        }
        let value = &tag[start..cursor];
        cursor += 1;
        if name == "id" {
            return Some(value);
        }
    }
}

fn fragment_matches_glyph_id(fragment: &str, gid: u16) -> bool {
    element_id(fragment).is_some_and(|id| id == format!("glyph{gid}") || id == gid.to_string())
}

fn collect_glyph_tagged_elements(source: &str, gid: u16) -> Vec<&str> {
    let mut elements = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = source[cursor..].find('<') {
        let start = cursor + relative;
        let Some(open_end) = find_svg_tag_end(source, start) else {
            break;
        };
        cursor = open_end;
        if !fragment_matches_glyph_id(&source[start..open_end], gid) {
            continue;
        }
        let Some(name_end) =
            source[start + 1..].find(|ch: char| ch.is_whitespace() || ch == '>' || ch == '/')
        else {
            continue;
        };
        let tag = &source[start + 1..start + 1 + name_end];
        if let Some(end) = find_balanced_tag_end(source, start, tag) {
            elements.push(&source[start..end]);
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
        assert!(!fragment_matches_glyph_id("<g data-id='glyph1'>", 1));
        assert!(!fragment_matches_glyph_id("<g id='glyph1-path'>", 1));
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

    #[test]
    fn svg_payload_rejects_active_content_and_external_references() {
        assert!(sanitize_svg_payload("<path d=\"M0 0\"/>").is_some());
        assert!(sanitize_svg_payload("<script>alert(1)</script>").is_none());
        assert!(sanitize_svg_payload("<path onclick=\"alert(1)\"/>").is_none());
        assert!(sanitize_svg_payload("<use href=\"https://example.test/x\"/>").is_none());
        assert!(sanitize_svg_payload("<path\nonload = \"alert(1)\"/>").is_none());
        assert!(sanitize_svg_payload("<use href = \" javascript:alert(1)\"/>").is_none());
        assert!(
            sanitize_svg_payload("<style>@import url(https://example.test);</style>").is_none()
        );
    }

    #[test]
    fn svg_document_respects_caller_decode_limit() {
        assert!(decode_svg_document(b"<svg/>", 5).is_none());
        assert!(decode_svg_document(b"<svg/>", 6).is_some());
    }
}
