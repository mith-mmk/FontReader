#[derive(Debug, Clone, Copy)]
pub(crate) struct DeltaSetIndexMap<'a> {
    data: &'a [u8],
}

impl<'a> DeltaSetIndexMap<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub(crate) fn map(&self, mut index: u32) -> Option<(u16, u16)> {
        let format = *self.data.first()?;
        let entry_format = *self.data.get(1)?;
        let mut cursor = 2usize;
        let map_count = if format == 0 {
            let value = u16::from_be_bytes([*self.data.get(cursor)?, *self.data.get(cursor + 1)?]);
            cursor += 2;
            value as u32
        } else {
            let value = u32::from_be_bytes([
                *self.data.get(cursor)?,
                *self.data.get(cursor + 1)?,
                *self.data.get(cursor + 2)?,
                *self.data.get(cursor + 3)?,
            ]);
            cursor += 4;
            value
        };

        if map_count == 0 {
            return None;
        }

        if index >= map_count {
            index = map_count - 1;
        }

        let entry_size = ((entry_format >> 4) & 0x03) as usize + 1;
        let inner_index_bit_count = ((entry_format & 0x0F) + 1) as u32;
        cursor = cursor.checked_add(entry_size.checked_mul(index as usize)?)?;

        let mut value = 0u32;
        for offset in 0..entry_size {
            value = (value << 8) + u32::from(*self.data.get(cursor + offset)?);
        }

        let outer_index = value >> inner_index_bit_count;
        let inner_mask = (1u32 << inner_index_bit_count).wrapping_sub(1);
        let inner_index = value & inner_mask;
        Some((outer_index as u16, inner_index as u16))
    }
}
