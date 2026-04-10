use std::io::{Error, ErrorKind};

#[derive(Debug, Clone)]
pub(crate) struct ItemVariationStore {
    data: Vec<u8>,
    data_offsets: Vec<u32>,
    regions: VariationRegionList,
}

#[derive(Debug, Clone, Default)]
struct VariationRegionList {
    axis_count: u16,
    regions: Vec<RegionAxisCoordinatesRecord>,
}

#[derive(Debug, Clone, Copy)]
struct RegionAxisCoordinatesRecord {
    start_coord: f32,
    peak_coord: f32,
    end_coord: f32,
}

impl ItemVariationStore {
    pub(crate) fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut cursor = 0usize;
        let format = read_u16(data, &mut cursor)?;
        if format != 1 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("unsupported ItemVariationStore format: {format}"),
            ));
        }

        let region_list_offset = read_u32(data, &mut cursor)? as usize;
        let count = read_u16(data, &mut cursor)? as usize;
        let mut data_offsets = Vec::with_capacity(count);
        for _ in 0..count {
            data_offsets.push(read_u32(data, &mut cursor)?);
        }

        let mut region_cursor = region_list_offset;
        let axis_count = read_u16(data, &mut region_cursor)?;
        let region_count = read_u16(data, &mut region_cursor)? as usize;
        let total_records = region_count
            .checked_mul(axis_count as usize)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidData,
                    "variation region record count overflow",
                )
            })?;
        let mut regions = Vec::with_capacity(total_records);
        for _ in 0..total_records {
            regions.push(RegionAxisCoordinatesRecord {
                start_coord: f2dot14_to_f32(read_i16(data, &mut region_cursor)?),
                peak_coord: f2dot14_to_f32(read_i16(data, &mut region_cursor)?),
                end_coord: f2dot14_to_f32(read_i16(data, &mut region_cursor)?),
            });
        }

        Ok(Self {
            data: data.to_vec(),
            data_offsets,
            regions: VariationRegionList {
                axis_count,
                regions,
            },
        })
    }

    pub(crate) fn parse_delta(
        &self,
        outer_index: u16,
        inner_index: u16,
        coordinates: &[f32],
    ) -> Option<f32> {
        let offset = *self.data_offsets.get(outer_index as usize)? as usize;
        let mut cursor = offset;
        let item_count = read_u16(&self.data, &mut cursor).ok()?;
        let word_delta_count = read_u16(&self.data, &mut cursor).ok()?;
        let region_index_count = read_u16(&self.data, &mut cursor).ok()?;

        if inner_index >= item_count {
            return None;
        }

        let mut region_indices = Vec::with_capacity(region_index_count as usize);
        for _ in 0..region_index_count {
            region_indices.push(read_u16(&self.data, &mut cursor).ok()?);
        }

        let has_long_words = (word_delta_count & 0x8000) != 0;
        let word_delta_count = word_delta_count & 0x7FFF;

        let mut delta_set_len = word_delta_count + region_index_count;
        if has_long_words {
            delta_set_len = delta_set_len.checked_mul(2)?;
        }

        cursor = cursor.checked_add(inner_index as usize * delta_set_len as usize)?;

        let mut delta = 0.0f32;
        let mut index = 0usize;
        while index < word_delta_count as usize {
            let region_index = *region_indices.get(index)?;
            let value = if has_long_words {
                read_i32(&self.data, &mut cursor).ok()? as f32
            } else {
                read_i16(&self.data, &mut cursor).ok()? as f32
            };
            delta += value * self.regions.evaluate_region(region_index, coordinates);
            index += 1;
        }

        while index < region_index_count as usize {
            let region_index = *region_indices.get(index)?;
            let value = if has_long_words {
                read_i16(&self.data, &mut cursor).ok()? as f32
            } else {
                read_i8(&self.data, &mut cursor).ok()? as f32
            };
            delta += value * self.regions.evaluate_region(region_index, coordinates);
            index += 1;
        }

        Some(delta)
    }

    pub(crate) fn region_scalars(&self, outer_index: u16, coordinates: &[f32]) -> Option<Vec<f32>> {
        let offset = *self.data_offsets.get(outer_index as usize)? as usize;
        let mut cursor = offset;
        let _item_count = read_u16(&self.data, &mut cursor).ok()?;
        let _word_delta_count = read_u16(&self.data, &mut cursor).ok()?;
        let region_index_count = read_u16(&self.data, &mut cursor).ok()? as usize;

        let mut scalars = Vec::with_capacity(region_index_count);
        for _ in 0..region_index_count {
            let region_index = read_u16(&self.data, &mut cursor).ok()?;
            scalars.push(self.regions.evaluate_region(region_index, coordinates));
        }
        Some(scalars)
    }

    pub(crate) fn region_index_count(&self, outer_index: u16) -> Option<usize> {
        let offset = *self.data_offsets.get(outer_index as usize)? as usize;
        let mut cursor = offset;
        let _item_count = read_u16(&self.data, &mut cursor).ok()?;
        let _word_delta_count = read_u16(&self.data, &mut cursor).ok()?;
        Some(read_u16(&self.data, &mut cursor).ok()? as usize)
    }
}

impl VariationRegionList {
    fn evaluate_region(&self, index: u16, coordinates: &[f32]) -> f32 {
        let mut scale = 1.0f32;
        for axis_index in 0..self.axis_count as usize {
            let coord = coordinates.get(axis_index).copied().unwrap_or(0.0);
            let offset = index as usize * self.axis_count as usize + axis_index;
            let Some(region) = self.regions.get(offset) else {
                return 0.0;
            };

            let factor = region.evaluate_axis(coord);
            if factor == 0.0 {
                return 0.0;
            }
            scale *= factor;
        }

        scale
    }
}

impl RegionAxisCoordinatesRecord {
    fn evaluate_axis(&self, coord: f32) -> f32 {
        let start = self.start_coord;
        let peak = self.peak_coord;
        let end = self.end_coord;

        if start > peak || peak > end {
            return 1.0;
        }

        if start < 0.0 && end > 0.0 && peak != 0.0 {
            return 1.0;
        }

        if peak == 0.0 || coord == peak {
            return 1.0;
        }

        if coord <= start || end <= coord {
            return 0.0;
        }

        if coord < peak {
            (coord - start) / (peak - start)
        } else {
            (end - coord) / (end - peak)
        }
    }
}

fn read_u16(data: &[u8], cursor: &mut usize) -> Result<u16, Error> {
    let bytes = read_bytes::<2>(data, cursor)?;
    Ok(u16::from_be_bytes(bytes))
}

fn read_i16(data: &[u8], cursor: &mut usize) -> Result<i16, Error> {
    let bytes = read_bytes::<2>(data, cursor)?;
    Ok(i16::from_be_bytes(bytes))
}

fn read_u32(data: &[u8], cursor: &mut usize) -> Result<u32, Error> {
    let bytes = read_bytes::<4>(data, cursor)?;
    Ok(u32::from_be_bytes(bytes))
}

fn read_i32(data: &[u8], cursor: &mut usize) -> Result<i32, Error> {
    let bytes = read_bytes::<4>(data, cursor)?;
    Ok(i32::from_be_bytes(bytes))
}

fn read_i8(data: &[u8], cursor: &mut usize) -> Result<i8, Error> {
    let value = *data
        .get(*cursor)
        .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "unexpected end of variation data"))?
        as i8;
    *cursor += 1;
    Ok(value)
}

fn read_bytes<const N: usize>(data: &[u8], cursor: &mut usize) -> Result<[u8; N], Error> {
    let end = cursor
        .checked_add(N)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "variation data offset overflow"))?;
    let slice = data
        .get(*cursor..end)
        .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "unexpected end of variation data"))?;
    let mut bytes = [0u8; N];
    bytes.copy_from_slice(slice);
    *cursor = end;
    Ok(bytes)
}

fn f2dot14_to_f32(value: i16) -> f32 {
    value as f32 / 16384.0
}
