use bin_rs::reader::BinaryReader;
use std::io::{Error, ErrorKind, SeekFrom};

#[derive(Debug, Clone)]
pub(crate) struct FVAR {
    pub(crate) axes: Vec<VariationAxisRecord>,
}

#[derive(Debug, Clone)]
pub(crate) struct VariationAxisRecord {
    pub(crate) tag: u32,
    pub(crate) min_value: f32,
    pub(crate) default_value: f32,
    pub(crate) max_value: f32,
    pub(crate) name_id: u16,
    pub(crate) hidden: bool,
}

impl FVAR {
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
                format!("unsupported fvar version: {version:#010x}"),
            ));
        }

        let axes_array_offset = read_u16(data, &mut cursor)? as usize;
        let _reserved = read_u16(data, &mut cursor)?;
        let axis_count = read_u16(data, &mut cursor)? as usize;
        let axis_size = read_u16(data, &mut cursor)? as usize;
        let instance_count = read_u16(data, &mut cursor)? as usize;
        let instance_size = read_u16(data, &mut cursor)? as usize;

        if axes_array_offset < 16 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "fvar axis array overlaps the table header",
            ));
        }

        if axis_size < 20 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "fvar axis record was smaller than 20 bytes",
            ));
        }

        let axis_end = axes_array_offset
            .checked_add(axis_count.checked_mul(axis_size).ok_or_else(|| {
                Error::new(ErrorKind::InvalidData, "fvar axis array size overflow")
            })?)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "fvar axis array offset overflow"))?;
        if axis_end > data.len() {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "fvar axis array exceeds table bounds",
            ));
        }
        let minimum_instance_size = 4usize
            .checked_add(axis_count.checked_mul(4).ok_or_else(|| {
                Error::new(ErrorKind::InvalidData, "fvar instance size overflow")
            })?)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "fvar instance size overflow"))?;
        if instance_count > 0 && instance_size < minimum_instance_size {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "fvar instance record is smaller than its required coordinates",
            ));
        }
        let instance_end = axis_end
            .checked_add(instance_count.checked_mul(instance_size).ok_or_else(|| {
                Error::new(ErrorKind::InvalidData, "fvar instance array size overflow")
            })?)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "fvar instance array offset overflow"))?;
        if instance_end > data.len() {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "fvar instance array exceeds table bounds",
            ));
        }

        let mut axes = Vec::with_capacity(axis_count);
        let mut axis_cursor = axes_array_offset;
        for _ in 0..axis_count {
            let tag = read_u32(data, &mut axis_cursor)?;
            let min_value = fixed_to_f32(read_i32(data, &mut axis_cursor)?);
            let default_value = fixed_to_f32(read_i32(data, &mut axis_cursor)?);
            let max_value = fixed_to_f32(read_i32(data, &mut axis_cursor)?);
            if min_value > default_value || default_value > max_value {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "fvar axis range does not contain its default value",
                ));
            }
            let flags = read_u16(data, &mut axis_cursor)?;
            let name_id = read_u16(data, &mut axis_cursor)?;
            if axis_size > 20 {
                axis_cursor = axis_cursor.checked_add(axis_size - 20).ok_or_else(|| {
                    Error::new(ErrorKind::InvalidData, "fvar axis cursor overflow")
                })?;
            }

            axes.push(VariationAxisRecord {
                tag,
                min_value,
                default_value,
                max_value,
                name_id,
                hidden: flags & 0x0001 != 0,
            });
        }

        Ok(Self { axes })
    }
}

impl VariationAxisRecord {
    pub(crate) fn normalized_value(&self, value: f32) -> f32 {
        let value = value.clamp(self.min_value, self.max_value);
        if value == self.default_value {
            0.0
        } else if value < self.default_value {
            let span = self.default_value - self.min_value;
            if span == 0.0 {
                0.0
            } else {
                (value - self.default_value) / span
            }
        } else {
            let span = self.max_value - self.default_value;
            if span == 0.0 {
                0.0
            } else {
                (value - self.default_value) / span
            }
        }
    }
}

fn read_u16(data: &[u8], cursor: &mut usize) -> Result<u16, Error> {
    let bytes = read_bytes::<2>(data, cursor)?;
    Ok(u16::from_be_bytes(bytes))
}

fn read_u32(data: &[u8], cursor: &mut usize) -> Result<u32, Error> {
    let bytes = read_bytes::<4>(data, cursor)?;
    Ok(u32::from_be_bytes(bytes))
}

fn read_i32(data: &[u8], cursor: &mut usize) -> Result<i32, Error> {
    let bytes = read_bytes::<4>(data, cursor)?;
    Ok(i32::from_be_bytes(bytes))
}

fn read_bytes<const N: usize>(data: &[u8], cursor: &mut usize) -> Result<[u8; N], Error> {
    let end = cursor
        .checked_add(N)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "fvar offset overflow"))?;
    let slice = data
        .get(*cursor..end)
        .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "unexpected end of fvar data"))?;
    let mut bytes = [0u8; N];
    bytes.copy_from_slice(slice);
    *cursor = end;
    Ok(bytes)
}

fn fixed_to_f32(value: i32) -> f32 {
    value as f32 / 65536.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed(value: i32) -> [u8; 4] {
        (value << 16).to_be_bytes()
    }

    fn axis_record(minimum: i32, default: i32, maximum: i32, flags: u16) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"wght");
        bytes.extend_from_slice(&fixed(minimum));
        bytes.extend_from_slice(&fixed(default));
        bytes.extend_from_slice(&fixed(maximum));
        bytes.extend_from_slice(&flags.to_be_bytes());
        bytes.extend_from_slice(&256u16.to_be_bytes());
        bytes
    }

    #[test]
    fn parses_hidden_axis_flag_from_bit_zero() {
        let mut bytes = vec![
            0x00, 0x01, 0x00, 0x00, // version 1.0
            0x00, 0x10, // axesArrayOffset
            0x00, 0x00, // reserved
            0x00, 0x01, // axisCount
            0x00, 0x14, // axisSize
            0x00, 0x00, // instanceCount
            0x00, 0x00, // instanceSize
        ];
        bytes.extend_from_slice(&axis_record(0, 1, 2, 1));

        let fvar = FVAR::from_bytes(&bytes).expect("valid fvar");
        assert!(fvar.axes[0].hidden);
        assert_eq!(fvar.axes[0].min_value, 0.0);
        assert_eq!(fvar.axes[0].default_value, 1.0);
        assert_eq!(fvar.axes[0].max_value, 2.0);
    }

    #[test]
    fn rejects_axis_range_without_default_value() {
        let mut bytes = vec![
            0x00, 0x01, 0x00, 0x00,
            0x00, 0x10,
            0x00, 0x00,
            0x00, 0x01,
            0x00, 0x14,
            0x00, 0x00,
            0x00, 0x00,
        ];
        bytes.extend_from_slice(&axis_record(2, 1, 3, 0));

        assert!(FVAR::from_bytes(&bytes).is_err());
    }

    #[test]
    fn rejects_truncated_instance_array() {
        let mut bytes = vec![
            0x00, 0x01, 0x00, 0x00,
            0x00, 0x10,
            0x00, 0x00,
            0x00, 0x01,
            0x00, 0x14,
            0x00, 0x01,
            0x00, 0x18,
        ];
        bytes.extend_from_slice(&axis_record(0, 1, 2, 0));

        assert!(FVAR::from_bytes(&bytes).is_err());
    }

    #[test]
    fn rejects_instance_record_without_all_axis_coordinates() {
        let mut bytes = vec![
            0x00, 0x01, 0x00, 0x00,
            0x00, 0x10,
            0x00, 0x00,
            0x00, 0x01,
            0x00, 0x14,
            0x00, 0x01,
            0x00, 0x08,
        ];
        bytes.extend_from_slice(&axis_record(0, 1, 2, 0));

        assert!(FVAR::from_bytes(&bytes).is_err());
    }
}
