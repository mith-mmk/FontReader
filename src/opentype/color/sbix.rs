#![allow(dead_code)]

use std::io::SeekFrom;

use crate::util::sniff_encoded_image_dimensions;
use base64::{engine::general_purpose, Engine as _};
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

#[derive(Debug, Clone)]
pub(crate) struct RasterGlyphData {
    pub(crate) offset_x: f32,
    pub(crate) offset_y: f32,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) graphic_type: u32,
    pub(crate) glyph_data: Vec<u8>,
}

impl SBIX {
    fn resolve_raster_glyph_in_strike(
        strike: &Strike,
        gid: usize,
        scale: f32,
        depth: usize,
    ) -> Option<RasterGlyphData> {
        if depth > strike.glyph_data.len() {
            return None;
        }

        let glyph_data = strike.glyph_data.get(gid)?.as_ref()?;
        if glyph_data.graphic_type == u32::from_be_bytes(*b"dupe") {
            if glyph_data.glyph_data.len() < 2 {
                return None;
            }
            let target_gid =
                u16::from_be_bytes([glyph_data.glyph_data[0], glyph_data.glyph_data[1]]) as usize;
            if target_gid == gid {
                return None;
            }
            let mut raster =
                Self::resolve_raster_glyph_in_strike(strike, target_gid, scale, depth + 1)?;
            raster.offset_x = glyph_data.original_offset_x as f32 * scale;
            raster.offset_y = glyph_data.original_offset_y as f32 * scale;
            return Some(raster);
        }

        let (width, height) = sniff_encoded_image_dimensions(&glyph_data.glyph_data)
            .map(|(_, width, height)| {
                let width = ((width as f32) * scale).round().max(1.0) as u32;
                let height = ((height as f32) * scale).round().max(1.0) as u32;
                (width, height)
            })
            .map_or((None, None), |(width, height)| (Some(width), Some(height)));

        Some(RasterGlyphData {
            offset_x: glyph_data.original_offset_x as f32 * scale,
            offset_y: glyph_data.original_offset_y as f32 * scale,
            width,
            height,
            graphic_type: glyph_data.graphic_type,
            glyph_data: glyph_data.glyph_data.clone(),
        })
    }

    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        length: u32,
        num_glyphs: u32,
    ) -> Result<Self, std::io::Error> {
        let offset = offset as u64;
        let length = length as u64;
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
        for (index, strike_offset) in strike_offsets.iter().enumerate() {
            if *strike_offset as u64 >= length {
                continue;
            }
            let next_offset = strike_offsets.get(index + 1).copied().unwrap_or(length) as u64;
            if next_offset <= *strike_offset || next_offset > length {
                continue;
            }

            if let Ok(strike) = Strike::new(
                reader,
                *strike_offset + offset,
                (next_offset - *strike_offset) as usize,
                num_glyphs,
            ) {
                strikes.push(strike);
            }
        }

        if num_strikes > 0 && strikes.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "sbix table does not contain any valid strikes",
            ));
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

    fn select_strike(&self, font_size: f32, fontunit: &str) -> Option<&Strike> {
        let mut result = self.strikes.last()?;
        for strike in self.strikes.iter() {
            if fontunit == "pt" {
                let ppem = strike.ppem as f32 * 72.0 / 96.0;
                if ppem > font_size {
                    result = strike;
                    break;
                }
            } else if strike.ppem as f32 > font_size {
                result = strike;
                break;
            }
        }
        Some(result)
    }

    pub(crate) fn get_raster_glyph(
        &self,
        gid: u32,
        font_size: f32,
        fontunit: &str,
    ) -> Option<RasterGlyphData> {
        let strike = self.select_strike(font_size, fontunit)?;
        let requested_ppem = if fontunit == "pt" {
            font_size * 96.0 / 72.0
        } else {
            font_size
        };
        let scale = if strike.ppem == 0 {
            1.0
        } else {
            requested_ppem / strike.ppem as f32
        };
        Self::resolve_raster_glyph_in_strike(strike, gid as usize, scale, 0)
    }

    pub(crate) fn get_svg(
        &self,
        gid: u32,
        fontsize: f64,
        fontunit: &str,
        _: &crate::fontreader::FontLayout,
        _: f64,
        _: f64,
    ) -> Option<String> {
        let glyph_data = self.get_raster_glyph(gid, fontsize as f32, fontunit)?;
        let width = glyph_data
            .width
            .map(|value| value.to_string())
            .unwrap_or_else(|| format!("{}{}", fontsize, fontunit));
        let height = glyph_data
            .height
            .map(|value| value.to_string())
            .unwrap_or_else(|| width.clone());
        let binary = &glyph_data.glyph_data;
        let bytes = u32::to_be_bytes(glyph_data.graphic_type);
        let mut base64 = general_purpose::STANDARD.encode(binary);
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
                let mut string = format!(
                    "<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" width=\"{}\" height=\"{}\" >\n",
                    width, height);
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
        strike_length: usize,
        num_glyphs: u32,
    ) -> Result<Self, std::io::Error> {
        let min_length = 4usize
            .checked_add((num_glyphs as usize + 1).saturating_mul(4))
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "sbix strike size overflow")
            })?;
        if strike_length < min_length {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "sbix strike is smaller than its glyph offset array",
            ));
        }

        reader.seek(SeekFrom::Start(offset))?;
        let ppem = reader.read_u16_be()?;
        let ppi = reader.read_u16_be()?;
        let mut glyph_data_offsets = Vec::new();
        for _ in 0..=num_glyphs {
            let glyph_data_offset = reader.read_u32_be()?;
            glyph_data_offsets.push(glyph_data_offset as usize);
        }

        let glyph_data_start = 4 + (num_glyphs as usize + 1) * 4;
        if glyph_data_offsets.iter().any(|glyph_data_offset| {
            *glyph_data_offset < glyph_data_start || *glyph_data_offset > strike_length
        }) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "sbix glyph offset is outside of the strike",
            ));
        }
        if glyph_data_offsets
            .windows(2)
            .any(|window| window[0] > window[1])
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "sbix glyph offsets are not sorted",
            ));
        }

        let mut glyph_data = Vec::new();
        for i in 0..glyph_data_offsets.len() - 1 {
            let length = glyph_data_offsets[i + 1] as isize - glyph_data_offsets[i] as isize;
            if length == 0 {
                glyph_data.push(None);
                continue;
            }
            if length < 8 {
                glyph_data.push(None);
                continue;
            }
            let glyph_offset = offset + glyph_data_offsets[i] as u64;
            match GlyphData::new(reader, glyph_offset, length as usize) {
                Ok(glyph) => glyph_data.push(Some(glyph)),
                Err(_) => glyph_data.push(None),
            }
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
