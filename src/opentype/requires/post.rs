use bin_rs::reader::BinaryReader;
use std::{fmt, io::SeekFrom};

// post table for PostScript

#[derive(Debug, Clone)]
pub(crate) struct POST {
    pub(crate) version: u32,
    pub(crate) italic_angle: i32,
    pub(crate) underline_position: i16,
    pub(crate) underline_thickness: i16,
    pub(crate) is_fixed_pitch: u32,
    pub(crate) min_mem_type42: u32,
    pub(crate) max_mem_type42: u32,
    pub(crate) min_mem_type1: u32,
    pub(crate) max_mem_type1: u32,
    // version 2.0
    pub(crate) number_of_glyphs: u16,
    pub(crate) glyph_name_index: Vec<u16>,
    pub(crate) names: Vec<String>,
}

impl fmt::Display for POST {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl POST {
    pub(crate) fn new<R: BinaryReader>(file: &mut R, offest: u32, length: u32) -> Self {
        get_post(file, offest, length)
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = "post\n".to_string();
        let mager_version = self.version >> 16;
        let minor_version = self.version & 0xFFFF;
        let version = format!("Version {}.{:04X}\n", mager_version, minor_version);
        string += &version;
        let italic_angle = format!("Italic Angle {}\n", self.italic_angle);
        string += &italic_angle;
        let underline_position = format!("Underline Position {}\n", self.underline_position);
        string += &underline_position;
        let underline_thickness = format!("Underline Thickness {}\n", self.underline_thickness);
        string += &underline_thickness;
        let is_fixed_pitch = format!("Is Fixed Pitch {}\n", self.is_fixed_pitch);
        string += &is_fixed_pitch;
        let min_mem_type42 = format!("Min Mem Type42 {}\n", self.min_mem_type42);
        string += &min_mem_type42;
        let max_mem_type42 = format!("Max Mem Type42 {}\n", self.max_mem_type42);
        string += &max_mem_type42;
        let min_mem_type1 = format!("Min Mem Type1 {}\n", self.min_mem_type1);
        string += &min_mem_type1;
        let max_mem_type1 = format!("Max Mem Type1 {}\n", self.max_mem_type1);
        string += &max_mem_type1;
        string
    }
}

fn get_post<R: BinaryReader>(file: &mut R, offest: u32, length: u32) -> POST {
    let file = file;
    let _buffer = vec![0u8; length as usize];
    file.seek(SeekFrom::Start(offest as u64)).unwrap();
    let version = file.read_u32_be().unwrap();
    let italic_angle = file.read_i32_be().unwrap();
    let underline_position = file.read_i16_be().unwrap();
    let underline_thickness = file.read_i16_be().unwrap();
    let is_fixed_pitch = file.read_u32_be().unwrap();
    let min_mem_type42 = file.read_u32_be().unwrap();
    let max_mem_type42 = file.read_u32_be().unwrap();
    let min_mem_type1 = file.read_u32_be().unwrap();
    let max_mem_type1 = file.read_u32_be().unwrap();

    let mut number_of_glyphs = 0;
    let mut glyph_name_index = Vec::new();
    let mut names = Vec::new();
    let remain = length - 32;
    if remain > 0 && version >= 0x0002_0000 {
        number_of_glyphs = file.read_u16_be().unwrap();
        for _ in 0..number_of_glyphs {
            let index = file.read_u16_be().unwrap();
            glyph_name_index.push(index);
        }
        let remain = (length - 34 - number_of_glyphs as u32 * 2) as usize;
        let buf = file.read_bytes_as_vec(remain).unwrap();
        let mut offset: usize = 0;
        while offset < buf.len() {
            let mut name = String::new();
            // PASCAL String
            let len = buf[offset];
            if offset + len as usize + 1 > buf.len() {
                break;
            }
            for i in 0..len {
                let c = buf[offset + i as usize + 1];
                name.push(c as char);
            }
            offset += len as usize + 1;
            names.push(name);
        }
    }

    POST {
        version,
        italic_angle,
        underline_position,
        underline_thickness,
        is_fixed_pitch,
        min_mem_type42,
        max_mem_type42,
        min_mem_type1,
        max_mem_type1,
        number_of_glyphs,
        glyph_name_index,
        names,
    }
}
