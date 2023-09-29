use std::io::SeekFrom;

use bin_rs::reader::BinaryReader;


#[derive(Debug, Clone)]
pub(crate) struct DeviceTable {
    pub(crate) start_size: u16,
    pub(crate) end_size: u16,
    pub(crate) delta_format: u16,
    pub(crate) delta_value: Vec<u16>,
}

impl DeviceTable {
    pub(crate) fn new<R: BinaryReader>(reader: &mut R, offset: u64) -> Result<Self,std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let start_size = reader.read_u16()?;
        let end_size = reader.read_u16()?;
        let delta_format = reader.read_u16()?;
        let delta_value = match delta_format {
            1 => { // LOCAL_2_BIT_DELTAS
                let length = ((end_size - start_size + 1) * 2) + 7 / 8;
                let mut delta_value = Vec::new();
                for _ in 0..length {
                    delta_value.push(reader.read_u16()?);
                }
                delta_value
            },
            2 => { // LOCAL_4_BIT_DELTAS
              let length = ((end_size - start_size + 1) * 4) + 7 / 8;
              let mut delta_value = Vec::new();
                for _ in 0..length {
                    delta_value.push(reader.read_u16()?);
                }
                delta_value
            },
            3 => { // LOCAL_8_BIT_DELTAS
              let length = (end_size - start_size + 1)  + 1 / 2;
                let mut delta_value = Vec::new();
                for _ in 0..length {
                    delta_value.push(reader.read_u16()?);
                }
                delta_value
            },
            _ => {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid delta format"));
            }
        };

        Ok(Self {
            start_size,
            end_size,
            delta_format,
            delta_value,
        })
    }
}