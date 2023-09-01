use std::{io::{Read, Seek, SeekFrom, Cursor}, fmt};
use bin_rs::reader::BinaryReader;

use crate::fontheader::{LONGDATETIME, self};

#[derive(Debug, Clone)]
pub(crate) struct HEAD {
  pub(crate) major_version: u16,
  pub(crate) minor_version: u16,
  pub(crate) font_revision: u32,
  pub(crate) check_sum_adjustment: u32,
  pub(crate) magic_number: u32,
  pub(crate) flags: u16,
  pub(crate) units_per_em: u16,
  pub(crate) created: LONGDATETIME,
  pub(crate) modified: LONGDATETIME,
  pub(crate) x_min: i16,
  pub(crate) y_min: i16,
  pub(crate) x_max: i16,
  pub(crate) y_max: i16,
  pub(crate) mac_style: u16,
  pub(crate) lowest_rec_ppem: u16,
  pub(crate) font_direction_hint: i16,
  pub(crate) index_to_loc_format: i16,
  pub(crate) glyph_data_format: i16,
}

impl fmt::Display for HEAD {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.to_string())
  }
}

impl HEAD{
  pub(crate) fn new<B:BinaryReader>(file: &mut B, offest: u32, length: u32) -> Self {
    get_head(file, offest, length)
  }

  pub(crate) fn to_string(&self) -> String {
    let mut string = "head\n".to_string();
    let version = format!("Version {}.{}\n", self.major_version, self.minor_version);
    string += &version;
    let font_revision = format!("Font Revision {}\n", self.font_revision);
    string += &font_revision;
    let check_sum_adjustment = format!("Check Sum Adjustment {:08X}\n", self.check_sum_adjustment);
    string += &check_sum_adjustment;
    let magic_number = format!("Magic Number {:08X}\n", self.magic_number);
    string += &magic_number;
    let flags = format!("Flags {:04X}\n", self.flags);
    string += &flags;
    let units_per_em = format!("Units Per EM {}\n", self.units_per_em);
    string += &units_per_em;
    let created = format!("Created {}\n", fontheader::longdatetime_to_string(&self.created));
    string += &created;
    let modified = format!("Modified {}\n", fontheader::longdatetime_to_string(&self.modified));
    string += &modified;
    let x_min = format!("xMin {}\n", self.x_min);
    string += &x_min;
    let y_min = format!("yMin {}\n", self.y_min);
    string += &y_min;
    let x_max = format!("xMax {}\n", self.x_max);
    string += &x_max;
    let y_max = format!("yMax {}\n", self.y_max);
    string += &y_max;
    let mut mac_style = format!("Mac Style {:04X} ", self.mac_style);
    if self.mac_style & 0x0001 == 0x0001 {
      mac_style += "Bold ";
    }
    if self.mac_style & 0x0002 == 0x0002 {
      mac_style += "Italic ";
    }
    if self.mac_style & 0x0004 == 0x0004 {
      mac_style += "Underline ";
    }
    if self.mac_style & 0x0008 == 0x0008 {
      mac_style += "Outline ";
    }
    if self.mac_style & 0x0010 == 0x0010 {
      mac_style += "Shadow ";
    }
    if self.mac_style & 0x0020 == 0x0020 {
      mac_style += "Condensed ";
    }
    if self.mac_style & 0x0040 == 0x0040 {
      mac_style += "Extended ";
    }
    mac_style += "\n";
    string += &mac_style;
    let lowest_rec_ppem = format!("Lowest Rec PPEM {}\n", self.lowest_rec_ppem);
    string += &lowest_rec_ppem;
    let font_direction_hint = format!("Font Direction Hint {}\n", self.font_direction_hint);
    string += &font_direction_hint;
    let index_to_loc_format = format!("Index To Loc Format {}\n", self.index_to_loc_format);
    string += &index_to_loc_format;
    let glyph_data_format = format!("Glyph Data Format {}\n", self.glyph_data_format);
    string += &glyph_data_format; 
    string
  }


}

fn get_head<R: BinaryReader>(file:&mut R, offest: u32, length: u32) -> HEAD {
  let mut file = file;
  file.seek(SeekFrom::Start(offest as u64)).unwrap();
  let buf = file.read_bytes_as_vec(length as usize).unwrap();

  let major_version = file.read_u16().unwrap();
  let minor_version = file.read_u16().unwrap();
  let font_revision = file.read_u32().unwrap();
  let check_sum_adjustment = file.read_u32().unwrap();
  let magic_number = file.read_u32().unwrap();
  let flags = file.read_u16().unwrap();
  let units_per_em = file.read_u16().unwrap();
  let created = file.read_i64().unwrap();
  let modified = file.read_i64().unwrap();
  let x_min = file.read_i16().unwrap();
  let y_min = file.read_i16().unwrap();
  let x_max = file.read_i16().unwrap();
  let y_max = file.read_i16().unwrap();
  let mac_style = file.read_u16().unwrap();
  let lowest_rec_ppem = file.read_u16().unwrap();
  let font_direction_hint = file.read_i16().unwrap();
  let index_to_loc_format = file.read_i16().unwrap();
  let glyph_data_format = file.read_i16().unwrap();
  HEAD {
    major_version,
    minor_version,
    font_revision,
    check_sum_adjustment,
    magic_number,
    flags,
    units_per_em,
    created,
    modified,
    x_min,
    y_min,
    x_max,
    y_max,
    mac_style,
    lowest_rec_ppem,
    font_direction_hint,
    index_to_loc_format,
    glyph_data_format,
  }

}



