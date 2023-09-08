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
    pub(crate) fn new<R: BinaryReader>(reader: &mut R, offset: u32, _: u32) -> Self {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let version = reader.read_u16_be().unwrap();
        let num_base_glyphs = reader.read_u16_be().unwrap();
        let base_glyph_records_offset = reader.read_u32_be().unwrap();
        let layer_records_offset = reader.read_u32_be().unwrap();
        let num_layers = reader.read_u16_be().unwrap();
        reader
            .seek(SeekFrom::Start((offset + base_glyph_records_offset) as u64))
            .unwrap();
        let mut base_glyph_records = Vec::new();
        for _ in 0..num_base_glyphs {
            let base_glyph_record = BaseGlyphRecord {
                base_glyph: reader.read_u16_be().unwrap(),
                first_layer_index: reader.read_u16_be().unwrap(),
                num_layers: reader.read_u16_be().unwrap(),
            };
            base_glyph_records.push(base_glyph_record);
        }
        reader
            .seek(SeekFrom::Start((offset + layer_records_offset) as u64))
            .unwrap();
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

    pub(crate) fn to_string(&self) -> String {
        let mut string = "COLR Table\n".to_string();
        string.push_str(&format!("version: {}\n", self.version));
        string.push_str(&format!("num_base_glyphs: {}\n", self.num_base_glyphs));
        string.push_str(&format!("num_layers: {}\n", self.num_layers));
        let max_length = 10;
        let len = if max_length < self.base_glyph_records.len() {
            max_length
        } else {
            self.base_glyph_records.len()
        };
        for i in 0..len {
            string.push_str(&format!(
                "base_glyph_record[{}]: {:?}\n",
                i, self.base_glyph_records[i]
            ));
        }
        let len = if max_length < self.layer_records.len() {
            max_length
        } else {
            self.layer_records.len()
        };
        for i in 0..len {
            string.push_str(&format!(
                "layer_record[{}]: {:?}\n",
                i, self.layer_records[i]
            ));
        }
        string
    }

    pub(crate) fn get_layer_record(&self, glyph_id: u16) -> Vec<LayerRecord> {
        let mut layer_records = Vec::new();
        let index = self
            .base_glyph_records
            .binary_search_by_key(&glyph_id, |base| base.base_glyph);
        if index.is_err() {
            return layer_records;
        }
        let base_glyph_record = &self.base_glyph_records[index.unwrap()];
        let num_layers = base_glyph_record.num_layers;
        let first_layer_index = base_glyph_record.first_layer_index;
        for i in 0..num_layers {
            layer_records.push(self.layer_records[(first_layer_index + i) as usize].clone());
        }
        layer_records
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
    pub(crate) glyph_id: u16,
    pub(crate) palette_index: u16, // see CPAL
}
