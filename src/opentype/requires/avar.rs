use bin_rs::reader::BinaryReader;
use std::io::{Error, ErrorKind, SeekFrom};

#[derive(Debug, Clone)]
pub(crate) struct AVAR {
    segment_maps: Vec<Vec<AxisValueMap>>,
}

#[derive(Debug, Clone, Copy)]
struct AxisValueMap {
    from_coordinate: f32,
    to_coordinate: f32,
}

impl AVAR {
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
                format!("unsupported avar version: {version:#010x}"),
            ));
        }

        let _reserved = read_u16(data, &mut cursor)?;
        let axis_count = read_u16(data, &mut cursor)? as usize;
        let mut segment_maps = Vec::with_capacity(axis_count);
        for _ in 0..axis_count {
            let map_count = read_u16(data, &mut cursor)? as usize;
            let mut map = Vec::with_capacity(map_count);
            for _ in 0..map_count {
                map.push(AxisValueMap {
                    from_coordinate: f2dot14_to_f32(read_i16(data, &mut cursor)?),
                    to_coordinate: f2dot14_to_f32(read_i16(data, &mut cursor)?),
                });
            }
            segment_maps.push(map);
        }

        Ok(Self { segment_maps })
    }

    pub(crate) fn map_coordinate(&self, coordinates: &mut [f32], coordinate_index: usize) {
        if coordinates.len() != self.segment_maps.len() {
            return;
        }
        let Some(map) = self.segment_maps.get(coordinate_index) else {
            return;
        };
        let Some(value) = coordinates.get_mut(coordinate_index) else {
            return;
        };
        if let Some(mapped) = map_value(map, *value) {
            *value = mapped;
        }
    }
}

fn map_value(map: &[AxisValueMap], value: f32) -> Option<f32> {
    if map.is_empty() {
        return Some(value);
    } else if map.len() == 1 {
        let record = map.first()?;
        return Some(value - record.from_coordinate + record.to_coordinate);
    }

    let record_0 = *map.first()?;
    if value <= record_0.from_coordinate {
        return Some(value - record_0.from_coordinate + record_0.to_coordinate);
    }

    let mut index = 1usize;
    while index < map.len() && value > map.get(index)?.from_coordinate {
        index += 1;
    }

    if index == map.len() {
        index -= 1;
    }

    let current = *map.get(index)?;
    if value >= current.from_coordinate {
        return Some(value - current.from_coordinate + current.to_coordinate);
    }

    let previous = *map.get(index - 1)?;
    if previous.from_coordinate == current.from_coordinate {
        return Some(previous.to_coordinate);
    }

    let denom = current.from_coordinate - previous.from_coordinate;
    let numerator =
        (current.to_coordinate - previous.to_coordinate) * (value - previous.from_coordinate);
    Some(previous.to_coordinate + numerator / denom)
}

fn read_u16(data: &[u8], cursor: &mut usize) -> Result<u16, Error> {
    let bytes = read_bytes::<2>(data, cursor)?;
    Ok(u16::from_be_bytes(bytes))
}

fn read_u32(data: &[u8], cursor: &mut usize) -> Result<u32, Error> {
    let bytes = read_bytes::<4>(data, cursor)?;
    Ok(u32::from_be_bytes(bytes))
}

fn read_i16(data: &[u8], cursor: &mut usize) -> Result<i16, Error> {
    let bytes = read_bytes::<2>(data, cursor)?;
    Ok(i16::from_be_bytes(bytes))
}

fn read_bytes<const N: usize>(data: &[u8], cursor: &mut usize) -> Result<[u8; N], Error> {
    let end = cursor
        .checked_add(N)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "avar offset overflow"))?;
    let slice = data
        .get(*cursor..end)
        .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "unexpected end of avar data"))?;
    let mut bytes = [0u8; N];
    bytes.copy_from_slice(slice);
    *cursor = end;
    Ok(bytes)
}

fn f2dot14_to_f32(value: i16) -> f32 {
    value as f32 / 16384.0
}
