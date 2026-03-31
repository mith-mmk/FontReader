use bin_rs::reader::BinaryReader;
use std::io::{Error, ErrorKind, SeekFrom};

use super::delta_set::DeltaSetIndexMap;
use super::var_store::ItemVariationStore;

#[derive(Debug, Clone)]
pub(crate) struct VVAR {
    data: Vec<u8>,
    variation_store: ItemVariationStore,
    advance_height_mapping_offset: Option<u32>,
    tsb_mapping_offset: Option<u32>,
    bsb_mapping_offset: Option<u32>,
    vorg_mapping_offset: Option<u32>,
}

impl VVAR {
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
                format!("unsupported VVAR version: {version:#010x}"),
            ));
        }

        let variation_store_offset = read_u32(data, &mut cursor)? as usize;
        let variation_store =
            ItemVariationStore::parse(data.get(variation_store_offset..).ok_or_else(|| {
                Error::new(ErrorKind::UnexpectedEof, "invalid VVAR store offset")
            })?)?;

        Ok(Self {
            data: data.to_vec(),
            variation_store,
            advance_height_mapping_offset: offset_to_option(read_u32(data, &mut cursor)?),
            tsb_mapping_offset: offset_to_option(read_u32(data, &mut cursor)?),
            bsb_mapping_offset: offset_to_option(read_u32(data, &mut cursor)?),
            vorg_mapping_offset: offset_to_option(read_u32(data, &mut cursor)?),
        })
    }

    pub(crate) fn advance_offset(&self, glyph_id: usize, coordinates: &[f32]) -> Option<f32> {
        let (outer_index, inner_index) = if let Some(offset) = self.advance_height_mapping_offset {
            DeltaSetIndexMap::new(self.data.get(offset as usize..)?).map(glyph_id as u32)?
        } else {
            (0, glyph_id as u16)
        };
        self.variation_store
            .parse_delta(outer_index, inner_index, coordinates)
    }

    pub(crate) fn top_side_bearing_offset(
        &self,
        glyph_id: usize,
        coordinates: &[f32],
    ) -> Option<f32> {
        let set_data = self.data.get(self.tsb_mapping_offset? as usize..)?;
        let (outer_index, inner_index) = DeltaSetIndexMap::new(set_data).map(glyph_id as u32)?;
        self.variation_store
            .parse_delta(outer_index, inner_index, coordinates)
    }

    #[allow(dead_code)]
    pub(crate) fn bottom_side_bearing_offset(
        &self,
        glyph_id: usize,
        coordinates: &[f32],
    ) -> Option<f32> {
        let set_data = self.data.get(self.bsb_mapping_offset? as usize..)?;
        let (outer_index, inner_index) = DeltaSetIndexMap::new(set_data).map(glyph_id as u32)?;
        self.variation_store
            .parse_delta(outer_index, inner_index, coordinates)
    }

    #[allow(dead_code)]
    pub(crate) fn vertical_origin_offset(
        &self,
        glyph_id: usize,
        coordinates: &[f32],
    ) -> Option<f32> {
        let set_data = self.data.get(self.vorg_mapping_offset? as usize..)?;
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
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "VVAR offset overflow"))?;
    let slice = data
        .get(*cursor..end)
        .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "unexpected end of VVAR data"))?;
    *cursor = end;
    Ok(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}
