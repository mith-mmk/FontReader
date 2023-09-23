use std::io::SeekFrom;

use bin_rs::reader::BinaryReader;

use crate::opentype::Glyph;
struct SBIX {
    version: u16,
    flags: u16,
    strikes: Vec<Strike>,
}

struct Strike {
    ppem: u16,
    ppi: u16,
    num_glyphs: u32,
    glyph_data_offsets: Vec<u64>,
    parsed_glyph_data: Option<Vec<GlyphData>>,
}

struct GlyphData {
    original_offset_x: i16,
    original_offset_y: i16,
    graphic_type: u32,
    glyph_data: Vec<u8>,
}

impl SBIX {
    pub(crate) fn new<R: BinaryReader>(reader: &mut R, offset: u32, num_glyphs: u32) -> Self {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let version = reader.read_u16_be().unwrap();
        let flags = reader.read_u16_be().unwrap();
        let num_strikes = reader.read_u32_be().unwrap();
        let mut strikes = Vec::new();
        for _ in 0..num_strikes {
            let strike_offset = reader.read_u32_be().unwrap();
            let strike_offset = strike_offset as u64 + offset as u64;
            strikes.push(Strike::new(reader, strike_offset, num_glyphs));
        }
        Self {
            version,
            flags,
            strikes,
        }
    }
}

impl Strike {
    pub(crate) fn new<R: BinaryReader>(reader: &mut R, offset: u64, num_glyphs: u32) -> Self {
        reader.seek(SeekFrom::Start(offset)).unwrap();
        let ppem = reader.read_u16_be().unwrap();
        let ppi = reader.read_u16_be().unwrap();
        let mut glyph_data_offsets = Vec::new();
        for _ in 0..num_glyphs {
            let glyph_data_offset = reader.read_u32_be().unwrap() as u64 + offset;
            glyph_data_offsets.push(glyph_data_offset);
        }

        Self {
            ppem,
            ppi,
            num_glyphs,
            glyph_data_offsets,
            parsed_glyph_data: None,
        }
    }

    pub(crate) fn parse<R: BinaryReader>(&mut self, reader: &mut R) {
        let mut parsed_glyph_data = Vec::new();
        for glyph_data_offset in self.glyph_data_offsets.iter() {
            reader.seek(SeekFrom::Start(*glyph_data_offset)).unwrap();
            parsed_glyph_data.push(GlyphData::new(reader));
        }
        self.parsed_glyph_data = Some(parsed_glyph_data);
    }
}

impl GlyphData {
    pub(crate) fn new<R: BinaryReader>(reader: &mut R) -> Self {
        let original_offset_x = reader.read_i16_be().unwrap();
        let original_offset_y = reader.read_i16_be().unwrap();
        let graphic_type = reader.read_u32_be().unwrap();
        let data_length = reader.read_u32_be().unwrap();
        let mut glyph_data = Vec::new();
        for _ in 0..data_length {
            glyph_data.push(reader.read_u8().unwrap());
        }
        Self {
            original_offset_x,
            original_offset_y,
            graphic_type,
            glyph_data,
        }
    }
}
