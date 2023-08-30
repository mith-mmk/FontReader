use std::{io::{Read, Seek, SeekFrom, Cursor}, fmt};
use byteorder::{BigEndian, ReadBytesExt};

// Postscript
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
  pub(crate) fn new<R:Read + Seek>(file: R, offest: u32, length: u32) -> Self {
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

fn get_post<R:Read + Seek>(file: R, offest: u32, length: u32) -> POST {
  let mut file = file;
  let mut buffer = vec![0u8; length as usize];
  file.seek(SeekFrom::Start(offest as u64)).unwrap();
  file.read_exact(&mut buffer).unwrap();
  let mut cursor = Cursor::new(buffer);
  let version = cursor.read_u32::<BigEndian>().unwrap();
  let italic_angle = cursor.read_i32::<BigEndian>().unwrap();
  let underline_position = cursor.read_i16::<BigEndian>().unwrap();
  let underline_thickness = cursor.read_i16::<BigEndian>().unwrap();
  let is_fixed_pitch = cursor.read_u32::<BigEndian>().unwrap();
  let min_mem_type42 = cursor.read_u32::<BigEndian>().unwrap();
  let max_mem_type42 = cursor.read_u32::<BigEndian>().unwrap();
  let min_mem_type1 = cursor.read_u32::<BigEndian>().unwrap();
  let max_mem_type1 = cursor.read_u32::<BigEndian>().unwrap();

  let mut number_of_glyphs = 0;
  let mut glyph_name_index = Vec::new();
  let mut names = Vec::new();
  let remain = length - 32;
  if remain > 0 {
    if version >= 0x0002_0000 {
      number_of_glyphs = cursor.read_u16::<BigEndian>().unwrap();
      for _ in 0..number_of_glyphs {
        let index = cursor.read_u16::<BigEndian>().unwrap();
        glyph_name_index.push(index);
      }
      let remain = (length - 34 - number_of_glyphs as u32 * 2) as usize;
      let mut buf = vec![0u8; remain];
      cursor.read_exact(&mut buf).unwrap();
      let mut offset: usize = 0;
      while offest < remain as u32 {
        let mut name = String::new();
        // PASCAL String
        let len = buf[offset];
        for i in 0..len {
          let c = buf[offset + i as usize + 1];
          name.push(c as char);
        }
        offset += len as usize + 1;
        names.push(name);
  
      }
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
