#![allow(dead_code)]

use std::io::SeekFrom;

use bin_rs::reader::BinaryReader;

#[derive(Debug, Clone)]
pub(crate) struct DeviceTable {
    pub(crate) start_size: u16,
    pub(crate) end_size: u16,
    pub(crate) delta_format: u16,
    pub(crate) delta_value: Vec<u16>,
    pub(crate) variation_index: Option<VariationIndex>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VariationIndex {
    pub(crate) delta_set_outer_index: u16,
    pub(crate) delta_set_inner_index: u16,
}

impl DeviceTable {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let start_size = reader.read_u16()?;
        let end_size = reader.read_u16()?;
        let delta_format = reader.read_u16()?;
        let (delta_value, variation_index) = match delta_format {
            1 => {
                // LOCAL_2_BIT_DELTAS
                let value_count = end_size.saturating_sub(start_size) as usize + 1;
                let length = ((value_count * 2) + 15) / 16;
                let mut delta_value = Vec::new();
                for _ in 0..length {
                    delta_value.push(reader.read_u16()?);
                }
                (delta_value, None)
            }
            2 => {
                // LOCAL_4_BIT_DELTAS
                let value_count = end_size.saturating_sub(start_size) as usize + 1;
                let length = ((value_count * 4) + 15) / 16;
                let mut delta_value = Vec::new();
                for _ in 0..length {
                    delta_value.push(reader.read_u16()?);
                }
                (delta_value, None)
            }
            3 => {
                // LOCAL_8_BIT_DELTAS
                let value_count = end_size.saturating_sub(start_size) as usize + 1;
                let length = (value_count + 1) / 2;
                let mut delta_value = Vec::new();
                for _ in 0..length {
                    delta_value.push(reader.read_u16()?);
                }
                (delta_value, None)
            }
            0x8000 => {
                // VariationIndex table shares the same offsets as DeviceTable.
                let variation_index = VariationIndex {
                    delta_set_outer_index: start_size,
                    delta_set_inner_index: end_size,
                };
                (Vec::new(), Some(variation_index))
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid delta format",
                ));
            }
        };

        Ok(Self {
            start_size,
            end_size,
            delta_format,
            delta_value,
            variation_index,
        })
    }

    pub(crate) fn is_variation_index(&self) -> bool {
        self.variation_index.is_some()
    }
}
