use std::io::SeekFrom;

use bin_rs::reader::BinaryReader;

#[derive(Debug, Clone)]
pub(crate) struct COLR {
  version: u16,
  num_base_glyphs: u16,
  base_glyph_records: Vec<BaseGlyphRecord>,
  layer_records: Vec<LayerRecord>,
  num_layers: u16,
  // version 1
  /*
  layers: Vec<Layer>,
  clip_list: Vec<u16>,
  ver_index_maps: Vec<VerIndexMap>,
  item_variations_store: ItemVariationsStore,
   */
}

impl COLR {
  pub(crate) fn new<R: BinaryReader>(reader:&mut R, offset: u32, _: u32) -> Self {
    reader.seek(SeekFrom::Start(offset as u64)).unwrap();
    let version = reader.read_u16_be().unwrap();
    let num_base_glyphs = reader.read_u16_be().unwrap();
    let num_layers = reader.read_u16_be().unwrap();
    let mut base_glyph_records = Vec::new();
    for _ in 0..num_base_glyphs {
      let base_glyph_record = BaseGlyphRecord {
        base_glyph: reader.read_u16_be().unwrap(),
        first_layer_index: reader.read_u16_be().unwrap(),
        num_layers: reader.read_u16_be().unwrap(),
      };
      base_glyph_records.push(base_glyph_record);
    }
    let mut layer_records = Vec::new();
    for _ in 0..num_layers {
      let layer_record = LayerRecord {
        glyph_id: reader.read_u16_be().unwrap(),
        palette_index: reader.read_u16_be().unwrap(),
      };
      layer_records.push(layer_record);
    }
    Self {
      version,
      num_base_glyphs,
      base_glyph_records,
      layer_records,
      num_layers,
    }
  }


}

#[derive(Debug, Clone)]

pub(crate) struct BaseGlyphRecord {
  base_glyph: u16,
  first_layer_index: u16,
  num_layers: u16,
}

#[derive(Debug, Clone)]

pub(crate) struct LayerRecord {
  glyph_id: u16,
  palette_index: u16, // see CPAL
}