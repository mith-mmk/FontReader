use bin_rs::reader::BinaryReader;
use std::io::{Error, ErrorKind, SeekFrom};

use super::delta_set::DeltaSetIndexMap;
use super::var_store::ItemVariationStore;

#[derive(Debug, Clone)]
pub(crate) struct HVAR {
    data: Vec<u8>,
    variation_store: ItemVariationStore,
    advance_width_mapping_offset: Option<u32>,
    lsb_mapping_offset: Option<u32>,
    rsb_mapping_offset: Option<u32>,
}

impl HVAR {
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
                format!("unsupported HVAR version: {version:#010x}"),
            ));
        }

        let variation_store_offset = read_u32(data, &mut cursor)? as usize;
        let variation_store =
            ItemVariationStore::parse(data.get(variation_store_offset..).ok_or_else(|| {
                Error::new(ErrorKind::UnexpectedEof, "invalid HVAR store offset")
            })?)?;

        Ok(Self {
            data: data.to_vec(),
            variation_store,
            advance_width_mapping_offset: offset_to_option(read_u32(data, &mut cursor)?),
            lsb_mapping_offset: offset_to_option(read_u32(data, &mut cursor)?),
            rsb_mapping_offset: offset_to_option(read_u32(data, &mut cursor)?),
        })
    }

    pub(crate) fn advance_offset(&self, glyph_id: usize, coordinates: &[f32]) -> Option<f32> {
        let (outer_index, inner_index) = if let Some(offset) = self.advance_width_mapping_offset {
            DeltaSetIndexMap::new(self.data.get(offset as usize..)?).map(glyph_id as u32)?
        } else {
            (0, glyph_id as u16)
        };
        self.variation_store
            .parse_delta(outer_index, inner_index, coordinates)
    }

    pub(crate) fn left_side_bearing_offset(
        &self,
        glyph_id: usize,
        coordinates: &[f32],
    ) -> Option<f32> {
        let set_data = self.data.get(self.lsb_mapping_offset? as usize..)?;
        let (outer_index, inner_index) = DeltaSetIndexMap::new(set_data).map(glyph_id as u32)?;
        self.variation_store
            .parse_delta(outer_index, inner_index, coordinates)
    }

    #[allow(dead_code)]
    pub(crate) fn right_side_bearing_offset(
        &self,
        glyph_id: usize,
        coordinates: &[f32],
    ) -> Option<f32> {
        let set_data = self.data.get(self.rsb_mapping_offset? as usize..)?;
        let (outer_index, inner_index) = DeltaSetIndexMap::new(set_data).map(glyph_id as u32)?;
        self.variation_store
            .parse_delta(outer_index, inner_index, coordinates)
    }
}

fn offset_to_option(value: u32) -> Option<u32> {
    if value == 0 {
        None
    } else {
        Some(value)
    }
}

fn read_u32(data: &[u8], cursor: &mut usize) -> Result<u32, Error> {
    let end = cursor
        .checked_add(4)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "HVAR offset overflow"))?;
    let slice = data
        .get(*cursor..end)
        .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "unexpected end of HVAR data"))?;
    *cursor = end;
    Ok(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}
