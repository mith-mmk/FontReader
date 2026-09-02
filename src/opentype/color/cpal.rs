#![allow(dead_code)]

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
        length: u32,
    ) -> Result<Self, std::io::Error> {
        let table_end = (offset as u64)
            .checked_add(length as u64)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "CPAL range overflow"))?;
        reader.seek(SeekFrom::Start(offset as u64))?;
        let version = reader.read_u16_be()?;
        let num_palette_entries = reader.read_u16_be()?;
        let num_palettes = reader.read_u16_be()?;
        let num_color_records = reader.read_u16_be()?;
        let color_records_array_offset = reader.read_u32_be()?;
        let index_array_end = (offset as u64)
            .checked_add(12)
            .and_then(|value| value.checked_add((num_palettes as u64).checked_mul(2)?))
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "CPAL index range overflow"))?;
        if index_array_end > table_end {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "CPAL palette indices exceed table bounds",
            ));
        }
        let color_records_start = (offset as u64)
            .checked_add(color_records_array_offset as u64)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "CPAL color record offset overflow"))?;
        let color_records_end = color_records_start
            .checked_add((num_color_records as u64).checked_mul(4).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "CPAL color record count overflow")
            })?)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "CPAL color record range overflow"))?;
        if color_records_start > table_end || color_records_end > table_end {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "CPAL color records exceed table bounds",
            ));
        }
        let mut color_record_indices = Vec::new();
        for _ in 0..num_palettes {
            let color_record_index = reader.read_u16_be()?;
            color_record_indices.push(color_record_index);
        }
        reader.seek(SeekFrom::Start(
            color_records_start,
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
        let max_length = 10;
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
    pub(crate) fn get_palette_color(
        &self,
        palette_index: usize,
        entry_index: u16,
    ) -> Option<ColorRecord> {
        if entry_index == 0xffff {
            return None;
        }
        if palette_index >= self.color_record_indices.len()
            || entry_index >= self.num_palette_entries
        {
            return None;
        }
        let record_index = (*self.color_record_indices.get(palette_index)? as usize)
            .checked_add(entry_index as usize)?;
        self.color_records.get(record_index).cloned()
    }

    pub(crate) fn get_pallet(&self, index: usize) -> Option<ColorRecord> {
        self.get_palette_color(0, u16::try_from(index).ok()?)
    }
}

#[derive(Debug, Clone)]

pub(crate) struct ColorRecord {
    pub(crate) red: u8,
    pub(crate) green: u8,
    pub(crate) blue: u8,
    pub(crate) alpha: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bin_rs::reader::BytesReader;

    #[test]
    fn palette_indices_are_read_once_and_resolved_relative_to_palette() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&2u16.to_be_bytes());
        bytes.extend_from_slice(&2u16.to_be_bytes());
        bytes.extend_from_slice(&4u16.to_be_bytes());
        bytes.extend_from_slice(&16u32.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&2u16.to_be_bytes());
        // CPAL stores color records as BGRA.
        bytes.extend_from_slice(&[0, 0, 255, 255]);
        bytes.extend_from_slice(&[0, 255, 0, 255]);
        bytes.extend_from_slice(&[255, 0, 0, 255]);
        bytes.extend_from_slice(&[255, 255, 255, 255]);

        let mut reader = BytesReader::new(&bytes);
        let cpal = CPAL::new(&mut reader, 0, bytes.len() as u32).expect("valid CPAL");
        assert_eq!(cpal.get_palette_color(0, 0).expect("palette 0").red, 255);
        assert_eq!(cpal.get_palette_color(1, 0).expect("palette 1").blue, 255);
        assert!(cpal.get_palette_color(0, 0xffff).is_none());
        assert!(cpal.get_palette_color(2, 0).is_none());
    }
}
