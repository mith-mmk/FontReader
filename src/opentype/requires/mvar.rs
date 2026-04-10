use bin_rs::reader::BinaryReader;
use std::io::{Error, ErrorKind, SeekFrom};

use super::var_store::ItemVariationStore;

#[derive(Debug, Clone)]
pub(crate) struct MVAR {
    variation_store: ItemVariationStore,
    records: Vec<ValueRecord>,
}

#[derive(Debug, Clone, Copy)]
struct ValueRecord {
    value_tag: u32,
    delta_set_outer_index: u16,
    delta_set_inner_index: u16,
}

impl MVAR {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        length: u32,
    ) -> Result<Self, Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let data = reader.read_bytes_as_vec(length as usize)?;
        Self::from_bytes(&data)
    }

    pub(crate) fn from_bytes(data: &[u8]) -> Result<Self, Error> {
        let mut cursor = 0usize;
        let version = read_u32(data, &mut cursor)?;
        if version != 0x0001_0000 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("unsupported MVAR version: {version:#010x}"),
            ));
        }

        let _reserved = read_u16(data, &mut cursor)?;
        let value_record_size = read_u16(data, &mut cursor)? as usize;
        if value_record_size != 8 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("unsupported MVAR value record size: {value_record_size}"),
            ));
        }

        let count = read_u16(data, &mut cursor)? as usize;
        if count == 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "MVAR record count was zero",
            ));
        }

        let variation_store_offset = read_u16(data, &mut cursor)? as usize;
        let variation_store =
            ItemVariationStore::parse(data.get(variation_store_offset..).ok_or_else(|| {
                Error::new(ErrorKind::UnexpectedEof, "invalid MVAR store offset")
            })?)?;

        let mut records = Vec::with_capacity(count);
        for _ in 0..count {
            records.push(ValueRecord {
                value_tag: read_u32(data, &mut cursor)?,
                delta_set_outer_index: read_u16(data, &mut cursor)?,
                delta_set_inner_index: read_u16(data, &mut cursor)?,
            });
        }
        records.sort_by_key(|record| record.value_tag);

        Ok(Self {
            variation_store,
            records,
        })
    }

    pub(crate) fn metric_offset(&self, tag: u32, coordinates: &[f32]) -> Option<f32> {
        let index = self
            .records
            .binary_search_by_key(&tag, |record| record.value_tag)
            .ok()?;
        let record = self.records.get(index)?;
        self.variation_store.parse_delta(
            record.delta_set_outer_index,
            record.delta_set_inner_index,
            coordinates,
        )
    }
}

fn read_u16(data: &[u8], cursor: &mut usize) -> Result<u16, Error> {
    let end = cursor
        .checked_add(2)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "MVAR offset overflow"))?;
    let slice = data
        .get(*cursor..end)
        .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "unexpected end of MVAR data"))?;
    *cursor = end;
    Ok(u16::from_be_bytes([slice[0], slice[1]]))
}

fn read_u32(data: &[u8], cursor: &mut usize) -> Result<u32, Error> {
    let end = cursor
        .checked_add(4)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "MVAR offset overflow"))?;
    let slice = data
        .get(*cursor..end)
        .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "unexpected end of MVAR data"))?;
    *cursor = end;
    Ok(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}
