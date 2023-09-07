use std::io::SeekFrom;

use bin_rs::reader::BinaryReader;

#[derive(Debug, Clone)]

pub(crate) struct CPAL {
  version: u16,
  num_palette_entries: u16,
  num_palettes: u16,
  num_color_records: u16,
  color_records: Vec<ColorRecord>,
  color_record_indices: Vec<u16>,
  // version 1
  /*
  palette_types: Vec<PaletteType>,
  palette_labels: Vec<PaletteLabel>,
  palette_entry_labels: Vec<PaletteEntryLabel>,
  */
}

impl CPAL {
  pub(crate) fn new<R: BinaryReader>(reader:&mut R, offset: u32, _: u32) -> Self {   
    reader.seek(SeekFrom::Start(offset as u64)).unwrap();
    let version = reader.read_u16_be().unwrap();
    let num_palette_entries = reader.read_u16_be().unwrap();
    let num_palettes = reader.read_u16_be().unwrap();
    let num_color_records = reader.read_u16_be().unwrap();
    let mut color_records = Vec::new();
    for _ in 0..num_color_records {
      let color_record = ColorRecord {
          red: reader.read_u8().unwrap(),
          green: reader.read_u8().unwrap(),
          blue: reader.read_u8().unwrap(),
          alpha: reader.read_u8().unwrap(),
      };

      color_records.push(color_record);
    }
    let mut color_record_indices = Vec::new();
    for _ in 0..num_palette_entries {
      color_record_indices.push(reader.read_u16_be().unwrap());
    }
    Self {
      version,
      num_palette_entries,
      num_palettes,
      num_color_records,
      color_records,
      color_record_indices,
    }


  }

}

#[derive(Debug, Clone)]

pub(crate) struct ColorRecord {
  pub(crate) red: u8,
  pub(crate) green: u8,
  pub(crate) blue: u8,
  pub(crate) alpha: u8,
}

