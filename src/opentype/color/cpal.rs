#![allow(dead_code)]

use std::io::SeekFrom;

use bin_rs::reader::BinaryReader;

#[derive(Debug, Clone)]
pub(crate) struct Cpal {
    version: u16,
    num_palette_entries: u16,
    num_palettes: u16,
    num_color_records: u16,
    color_records: Vec<ColorRecord>,
    color_record_indices: Vec<u16>,
    palette_types: Vec<u32>,
    palette_labels: Vec<u16>,
    palette_entry_labels: Vec<u16>,
}

impl Cpal {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        length: u32,
    ) -> Result<Self, std::io::Error> {
        let table_end = (offset as u64).checked_add(length as u64).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "CPAL range overflow")
        })?;
        reader.seek(SeekFrom::Start(offset as u64))?;
        let version = reader.read_u16_be()?;
        let num_palette_entries = reader.read_u16_be()?;
        let num_palettes = reader.read_u16_be()?;
        let num_color_records = reader.read_u16_be()?;
        let color_records_array_offset = reader.read_u32_be()?;
        let index_array_end = (offset as u64)
            .checked_add(12)
            .and_then(|value| value.checked_add((num_palettes as u64).checked_mul(2)?))
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "CPAL index range overflow")
            })?;
        if index_array_end > table_end {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "CPAL palette indices exceed table bounds",
            ));
        }
        let color_records_start = (offset as u64)
            .checked_add(color_records_array_offset as u64)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "CPAL color record offset overflow",
                )
            })?;
        let color_records_end = color_records_start
            .checked_add((num_color_records as u64).checked_mul(4).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "CPAL color record count overflow",
                )
            })?)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "CPAL color record range overflow",
                )
            })?;
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
        reader.seek(SeekFrom::Start(color_records_start))?;
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

        let mut palette_types = Vec::new();
        let mut palette_labels = Vec::new();
        let mut palette_entry_labels = Vec::new();
        if version == 1 {
            let v1_header_end = index_array_end.checked_add(12).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "CPAL v1 header overflow")
            })?;
            if v1_header_end > table_end {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "CPAL v1 header exceeds table bounds",
                ));
            }
            reader.seek(SeekFrom::Start(index_array_end))?;
            let palette_types_offset = reader.read_u32_be()?;
            let palette_labels_offset = reader.read_u32_be()?;
            let palette_entry_labels_offset = reader.read_u32_be()?;

            if palette_types_offset != 0 {
                let start = (offset as u64)
                    .checked_add(palette_types_offset as u64)
                    .ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "CPAL palette types offset overflow",
                        )
                    })?;
                let bytes_len = (num_palettes as u64).checked_mul(4).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "CPAL palette types length overflow",
                    )
                })?;
                let end = start.checked_add(bytes_len).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "CPAL palette types end overflow",
                    )
                })?;
                if end > table_end {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "CPAL palette types exceed table bounds",
                    ));
                }
                reader.seek(SeekFrom::Start(start))?;
                for _ in 0..num_palettes {
                    palette_types.push(reader.read_u32_be()?);
                }
            }

            if palette_labels_offset != 0 {
                let start = (offset as u64)
                    .checked_add(palette_labels_offset as u64)
                    .ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "CPAL palette labels offset overflow",
                        )
                    })?;
                let bytes_len = (num_palettes as u64).checked_mul(2).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "CPAL palette labels length overflow",
                    )
                })?;
                let end = start.checked_add(bytes_len).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "CPAL palette labels end overflow",
                    )
                })?;
                if end > table_end {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "CPAL palette labels exceed table bounds",
                    ));
                }
                reader.seek(SeekFrom::Start(start))?;
                for _ in 0..num_palettes {
                    palette_labels.push(reader.read_u16_be()?);
                }
            }

            if palette_entry_labels_offset != 0 {
                let start = (offset as u64)
                    .checked_add(palette_entry_labels_offset as u64)
                    .ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "CPAL palette entry labels offset overflow",
                        )
                    })?;
                let bytes_len = (num_palette_entries as u64).checked_mul(2).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "CPAL palette entry labels length overflow",
                    )
                })?;
                let end = start.checked_add(bytes_len).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "CPAL palette entry labels end overflow",
                    )
                })?;
                if end > table_end {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "CPAL palette entry labels exceed table bounds",
                    ));
                }
                reader.seek(SeekFrom::Start(start))?;
                for _ in 0..num_palette_entries {
                    palette_entry_labels.push(reader.read_u16_be()?);
                }
            }
        }

        Ok(Self {
            version,
            num_palette_entries,
            num_palettes,
            num_color_records,
            color_records,
            color_record_indices,
            palette_types,
            palette_labels,
            palette_entry_labels,
        })
    }

    fn dump_text(&self) -> String {
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
        for (i, record) in self.color_records[..len].iter().enumerate() {
            string.push_str(&format!(
                "color_record[{}]: {} {} {} {}\n",
                i, record.red, record.green, record.blue, record.alpha
            ));
        }

        let len = if 10 < self.color_record_indices.len() {
            max_length
        } else {
            self.color_record_indices.len()
        };
        for (i, index) in self.color_record_indices.iter().take(len).enumerate() {
            string.push_str(&format!("color_record_indices[{}]: {}\n", i, index));
        }
        string
    }
}

impl Cpal {
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

    pub(crate) fn version(&self) -> u16 {
        self.version
    }

    pub(crate) fn palette_type(&self, palette_index: usize) -> Option<u32> {
        self.palette_types.get(palette_index).copied()
    }

    pub(crate) fn palette_label(&self, palette_index: usize) -> Option<u16> {
        self.palette_labels.get(palette_index).copied()
    }

    pub(crate) fn palette_entry_label(&self, entry_index: u16) -> Option<u16> {
        self.palette_entry_labels.get(entry_index as usize).copied()
    }
}

#[derive(Debug, Clone)]

pub(crate) struct ColorRecord {
    pub(crate) red: u8,
    pub(crate) green: u8,
    pub(crate) blue: u8,
    pub(crate) alpha: u8,
}

impl std::fmt::Display for Cpal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bin_rs::reader::BytesReader;

    fn display_palette(num_palette_entries: u16, num_color_records: u16) -> Cpal {
        Cpal {
            version: 0,
            num_palette_entries,
            num_palettes: 1,
            num_color_records,
            color_records: vec![
                ColorRecord {
                    red: 1,
                    green: 2,
                    blue: 3,
                    alpha: 4,
                };
                num_color_records as usize
            ],
            color_record_indices: vec![0],
            palette_types: Vec::new(),
            palette_labels: Vec::new(),
            palette_entry_labels: Vec::new(),
        }
    }

    #[test]
    fn display_preserves_palette_entry_dependent_record_limit() {
        for (num_palette_entries, displayed_records) in [(10, 12), (11, 10)] {
            let cpal = display_palette(num_palette_entries, 12);
            let mut expected = format!(
                "CPAL Table\nversion: 0\nnum_palette_entries: {}\nnum_palettes: 1\nnum_color_records: 12\n",
                num_palette_entries
            );
            for i in 0..displayed_records {
                expected.push_str(&format!("color_record[{}]: 1 2 3 4\n", i));
            }
            expected.push_str("color_record_indices[0]: 0\n");
            assert_eq!(cpal.to_string(), expected);
        }
    }

    #[test]
    #[should_panic]
    fn display_preserves_panic_when_record_limit_exceeds_available_records() {
        let _ = display_palette(11, 9).to_string();
    }

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
        let cpal = Cpal::new(&mut reader, 0, bytes.len() as u32).expect("valid CPAL");
        assert_eq!(cpal.get_palette_color(0, 0).expect("palette 0").red, 255);
        assert_eq!(cpal.get_palette_color(1, 0).expect("palette 1").blue, 255);
        assert!(cpal.get_palette_color(0, 0xffff).is_none());
        assert!(cpal.get_palette_color(2, 0).is_none());
    }
}
