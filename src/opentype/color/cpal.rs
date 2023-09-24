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
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        _: u32,
    ) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let version = reader.read_u16_be()?;
        let num_palette_entries = reader.read_u16_be()?;
        let num_palettes = reader.read_u16_be()?;
        let num_color_records = reader.read_u16_be()?;
        let color_records_array_offset = reader.read_u32_be()?;
        let mut color_record_indices = Vec::new();
        let mut max_color_record_index = 0;
        for _ in 0..num_palettes {
            let color_record_index = reader.read_u16_be()?;
            if max_color_record_index < color_record_index {
                max_color_record_index = color_record_index;
            }
            color_record_indices.push(reader.read_u16_be()?);
        }
        reader.seek(SeekFrom::Start(
            (offset + color_records_array_offset) as u64,
        ))?;
        let mut color_records = Vec::new();
        for _ in 0..num_color_records {
            let color_record = ColorRecord {
                blue: reader.read_u8()?,
                green: reader.read_u8()?,
                red: reader.read_u8()?,
                alpha: reader.read_u8()?,
            };

            color_records.push(color_record);
        }

        Ok(Self {
            version,
            num_palette_entries,
            num_palettes,
            num_color_records,
            color_records,
            color_record_indices,
        })
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = "CPAL Table\n".to_string();
        string.push_str(&format!("version: {}\n", self.version));
        string.push_str(&format!(
            "num_palette_entries: {}\n",
            self.num_palette_entries
        ));
        string.push_str(&format!("num_palettes: {}\n", self.num_palettes));
        string.push_str(&format!("num_color_records: {}\n", self.num_color_records));
        let max_length =  10;
        let len = if max_length < self.num_palette_entries as usize {
            max_length
        } else {
            self.color_records.len()
        };
        for i in 0..len {
            string.push_str(&format!(
                "color_record[{}]: {} {} {} {}\n",
                i,
                self.color_records[i].red,
                self.color_records[i].green,
                self.color_records[i].blue,
                self.color_records[i].alpha
            ));
        }

        let len = if 10 < self.color_record_indices.len() {
            max_length
        } else {
            self.color_record_indices.len()
        };
        for i in 0..len {
            string.push_str(&format!(
                "color_record_indices[{}]: {}\n",
                i, self.color_record_indices[i]
            ));
        }
        string
    }
}

impl CPAL {
    pub(crate) fn get_pallet(&self, index: usize) -> ColorRecord {
        self.color_records[index].clone()
    }
}

#[derive(Debug, Clone)]

pub(crate) struct ColorRecord {
    pub(crate) red: u8,
    pub(crate) green: u8,
    pub(crate) blue: u8,
    pub(crate) alpha: u8,
}
