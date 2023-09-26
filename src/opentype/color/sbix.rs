use std::io::SeekFrom;

use base64::{
    engine::{general_purpose},
    Engine as _,
};
use bin_rs::reader::BinaryReader;

#[derive(Debug, Clone)]
pub(crate) struct SBIX {
    version: u16,
    flags: u16,
    strikes: Vec<Strike>,
}

#[derive(Debug, Clone)]
pub(crate) struct Strike {
    ppem: u16,
    ppi: u16,
    glyph_data: Vec<Option<GlyphData>>,
}

#[derive(Debug, Clone)]
pub(crate) struct GlyphData {
    original_offset_x: i16,
    original_offset_y: i16,
    graphic_type: u32,
    glyph_data: Vec<u8>,
}

impl SBIX {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        num_glyphs: u32,
    ) -> Result<Self, std::io::Error> {
        let offset = offset as u64;
        reader.seek(SeekFrom::Start(offset))?;
        let version = reader.read_u16_be()?;
        let flags = reader.read_u16_be()?;
        let num_strikes = reader.read_u32_be()?;
        let mut strikes = Vec::new();
        let mut strike_offsets = Vec::new();
        for _ in 0..num_strikes {
            let strike_offset = reader.read_u32_be()?;
            strike_offsets.push(strike_offset as u64);
        }
        for strike_offset in strike_offsets.iter() {
            strikes.push(Strike::new(reader, *strike_offset + offset, num_glyphs)?);
        }

        Ok(Self {
            version,
            flags,
            strikes,
        })
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = "SBIX\n".to_string();
        string += &format!("version: {}\n", self.version);
        string += &format!("flags: {}\n", self.flags);
        for (i, strike) in self.strikes.iter().enumerate() {
            string += &format!("strike {}\n", i);
            string += &format!("  ppem: {}\n", strike.ppem);
            string += &format!("  ppi: {}\n", strike.ppi);
            string += &format!("  num_glyphs: {}\n", strike.glyph_data.len());
            let mut some = 0;
            for (j, glyph_data) in strike.glyph_data.iter().enumerate() {
                if some > 10 {
                    break;
                }
                string += &format!("  glyph_data {}\n", j);
                if let Some(glyph_data) = glyph_data {
                    string += &format!("    original_offset_x: {}\n", glyph_data.original_offset_x);
                    string += &format!("    original_offset_y: {}\n", glyph_data.original_offset_y);
                    let u32bytes = u32::to_be_bytes(glyph_data.graphic_type.clone());
                    let tag = String::from_utf8_lossy(&u32bytes);
                    string += &format!("    graphic_type: {}\n", tag);
                    string += &format!("    glyph_data: {}\n", glyph_data.glyph_data.len());
                    some += 1;
                } else {
                    string += &format!("    None\n");
                }
            }
        }
        string
    }

    pub(crate) fn get_svg(
        &self,
        gid: u32,
        fonsize: f64,
        fontunit: &str,
        _: &crate::fontreader::HorizontalLayout,
        _: f64,
        _: f64,
    ) -> Option<String> {
        let strike = &self.strikes[self.strikes.len() - 1];
        let glyph_data = &strike.glyph_data[gid as usize];
        if glyph_data.is_none() {
            return None;
        }
        let width = format!("{}{}", fonsize, fontunit);
        let height = width.clone();
        let binary = &glyph_data.as_ref().unwrap().glyph_data;
        let bytes = u32::to_be_bytes(glyph_data.as_ref().unwrap().graphic_type);
        let mut base64 = general_purpose::STANDARD.encode(&binary);
        match &bytes {
            b"png " => {
                // base64
                base64 = format!("data:image/png;base64,{}", base64);
            }
            b"jpg " => {
                // base64
                base64 = format!("data:image/jpeg;base64,{}", base64);
            }
            _ => {
                let mut string = "<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" width=\"100%\" height=\"100%\" viewBox=\"0 0 1000 1000\">\n".to_string();
                string += "</svg>\n";
                return None;
            }
        }
        let string = format!(
            "<img width=\"{}\" height=\"{}\" src=\"{}\" />\n",
            width, height, base64
        );
        Some(string)
    }
}

impl Strike {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        num_glyphs: u32,
    ) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let ppem = reader.read_u16_be()?;
        let ppi = reader.read_u16_be()?;
        let mut glyph_data_offsets = Vec::new();
        for _ in 0..=num_glyphs {
            let glyph_data_offset = reader.read_u32_be()?;
            glyph_data_offsets.push(glyph_data_offset as usize);
        }

        let mut glyph_data = Vec::new();
        for i in 0..glyph_data_offsets.len() - 1 {
            let length = glyph_data_offsets[i + 1] as isize - glyph_data_offsets[i] as isize;
            // println!("i: {} {} {} {}", i, length, glyph_data_offsets[i + 1], glyph_data_offsets[i]);
            if length == 0 {
                glyph_data.push(None);
                continue;
            }
            let glyph_offset = offset + glyph_data_offsets[i] as u64;
            glyph_data.push(Some(GlyphData::new(reader, glyph_offset, length as usize)?));
        }

        Ok(Self {
            ppem,
            ppi,
            glyph_data,
        })
    }
}

impl GlyphData {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        length: usize,
    ) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let original_offset_x = reader.read_i16_be()?;
        let original_offset_y = reader.read_i16_be()?;
        let graphic_type = reader.read_u32_be()?;
        let glyph_data = reader.read_bytes_as_vec(length - 8)?;
        Ok(Self {
            original_offset_x,
            original_offset_y,
            graphic_type,
            glyph_data,
        })
    }
}
