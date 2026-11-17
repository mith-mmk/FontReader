//! Fail-closed ItemVariationStore subset for authored GDEF carets only.
//! Does not change the existing tolerant HVAR/VVAR/CFF variation paths.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VariationError {
    Malformed,
    InvalidIndex,
    AxisMismatch,
    InvalidCoordinates,
    ResourceLimit,
}

#[derive(Debug, Clone)]
pub(crate) struct CaretVariationStore {
    data: Vec<u8>,
    axis_count: usize,
    regions: Vec<Vec<[f32; 3]>>,
    items: Vec<Item>,
}

#[derive(Debug, Clone)]
struct Item {
    count: usize,
    long_words: bool,
    word_count: usize,
    regions: Vec<usize>,
    rows: usize,
    row_size: usize,
}

const MAX_WORK: usize = 1_000_000;
pub(crate) const MAX_STORE_BYTES: usize = 16 * 1024 * 1024;

fn span(data: &[u8], offset: usize, size: usize) -> Result<&[u8], VariationError> {
    data.get(offset..offset.checked_add(size).ok_or(VariationError::Malformed)?)
        .ok_or(VariationError::Malformed)
}
fn u16_at(data: &[u8], offset: usize) -> Result<u16, VariationError> {
    Ok(u16::from_be_bytes(
        span(data, offset, 2)?.try_into().unwrap(),
    ))
}
fn u32_at(data: &[u8], offset: usize) -> Result<u32, VariationError> {
    Ok(u32::from_be_bytes(
        span(data, offset, 4)?.try_into().unwrap(),
    ))
}
fn charge(budget: &mut usize, amount: usize) -> Result<(), VariationError> {
    *budget = budget
        .checked_sub(amount)
        .ok_or(VariationError::ResourceLimit)?;
    Ok(())
}

impl CaretVariationStore {
    pub(crate) fn parse(data: &[u8]) -> Result<Self, VariationError> {
        if data.len() > MAX_STORE_BYTES {
            return Err(VariationError::ResourceLimit);
        }
        if u16_at(data, 0)? != 1 {
            return Err(VariationError::Malformed);
        }
        let region_offset =
            usize::try_from(u32_at(data, 2)?).map_err(|_| VariationError::Malformed)?;
        let count = usize::from(u16_at(data, 6)?);
        let header_end = 8usize
            .checked_add(count.checked_mul(4).ok_or(VariationError::Malformed)?)
            .ok_or(VariationError::Malformed)?;
        span(data, 0, header_end)?;
        if region_offset < header_end {
            return Err(VariationError::Malformed);
        }
        let axis_count = usize::from(u16_at(data, region_offset)?);
        let region_count = usize::from(u16_at(data, region_offset + 2)?);
        let records = axis_count
            .checked_mul(region_count)
            .ok_or(VariationError::Malformed)?;
        let mut budget = MAX_WORK;
        charge(
            &mut budget,
            records
                .checked_add(region_count)
                .ok_or(VariationError::Malformed)?,
        )?;
        let region_size = 4usize
            .checked_add(records.checked_mul(6).ok_or(VariationError::Malformed)?)
            .ok_or(VariationError::Malformed)?;
        span(data, region_offset, region_size)?;
        let mut regions = Vec::with_capacity(region_count);
        let mut cursor = region_offset + 4;
        for _ in 0..region_count {
            let mut axes = Vec::with_capacity(axis_count);
            for _ in 0..axis_count {
                let mut tuple = [0.0; 3];
                for coordinate in &mut tuple {
                    *coordinate = f32::from(u16_at(data, cursor)? as i16) / 16384.0;
                    cursor += 2;
                }
                let [start, peak, end] = tuple;
                // Conservative subset: don't turn malformed regions into a
                // neutral scalar, even though tolerant shaping paths may do so.
                if tuple.iter().any(|c| !(-1.0..=1.0).contains(c))
                    || start > peak
                    || peak > end
                    || (start < 0.0 && end > 0.0 && peak != 0.0)
                {
                    return Err(VariationError::Malformed);
                }
                axes.push(tuple);
            }
            regions.push(axes);
        }
        let mut items = Vec::with_capacity(count);
        let mut intervals = vec![(region_offset, region_offset + region_size)];
        for outer in 0..count {
            let offset = usize::try_from(u32_at(data, 8 + outer * 4)?)
                .map_err(|_| VariationError::Malformed)?;
            if offset < header_end {
                return Err(VariationError::Malformed);
            }
            span(data, offset, 6)?;
            let item_count = usize::from(u16_at(data, offset)?);
            let packed = u16_at(data, offset + 2)?;
            let long_words = packed & 0x8000 != 0;
            let word_count = usize::from(packed & 0x7fff);
            let index_count = usize::from(u16_at(data, offset + 4)?);
            if word_count > index_count {
                return Err(VariationError::Malformed);
            }
            charge(&mut budget, index_count + 1)?;
            let rows = offset
                .checked_add(6)
                .and_then(|v| v.checked_add(index_count.checked_mul(2)?))
                .ok_or(VariationError::Malformed)?;
            span(data, offset, rows - offset)?;
            let mut indices = Vec::with_capacity(index_count);
            for i in 0..index_count {
                let index = usize::from(u16_at(data, offset + 6 + i * 2)?);
                if index >= region_count {
                    return Err(VariationError::InvalidIndex);
                }
                indices.push(index);
            }
            let row_size = index_count
                .checked_add(word_count)
                .and_then(|v| v.checked_mul(if long_words { 2 } else { 1 }))
                .ok_or(VariationError::Malformed)?;
            let bytes = item_count
                .checked_mul(row_size)
                .ok_or(VariationError::Malformed)?;
            span(data, rows, bytes)?;
            intervals.push((offset, rows + bytes));
            items.push(Item {
                count: item_count,
                long_words,
                word_count,
                regions: indices,
                rows,
                row_size,
            });
        }
        intervals.sort_unstable();
        for pair in intervals.windows(2) {
            if pair[0].1 > pair[1].0 && pair[0] != pair[1] {
                return Err(VariationError::Malformed);
            }
        }
        Ok(Self {
            data: data.to_vec(),
            axis_count,
            regions,
            items,
        })
    }

    pub(crate) fn delta(
        &self,
        outer: u16,
        inner: u16,
        coordinates: &[f32],
    ) -> Result<f32, VariationError> {
        if coordinates.len() != self.axis_count {
            return Err(VariationError::AxisMismatch);
        }
        if coordinates
            .iter()
            .any(|c| !c.is_finite() || !(-1.0..=1.0).contains(c))
        {
            return Err(VariationError::InvalidCoordinates);
        }
        let item = self
            .items
            .get(usize::from(outer))
            .ok_or(VariationError::InvalidIndex)?;
        if usize::from(inner) >= item.count {
            return Err(VariationError::InvalidIndex);
        }
        if item
            .regions
            .len()
            .checked_mul(self.axis_count)
            .ok_or(VariationError::ResourceLimit)?
            > MAX_WORK
        {
            return Err(VariationError::ResourceLimit);
        }
        let mut cursor = item
            .rows
            .checked_add(
                usize::from(inner)
                    .checked_mul(item.row_size)
                    .ok_or(VariationError::Malformed)?,
            )
            .ok_or(VariationError::Malformed)?;
        let mut result = 0.0f64;
        for (i, &region) in item.regions.iter().enumerate() {
            let size = match (item.long_words, i < item.word_count) {
                (true, true) => 4,
                (true, false) | (false, true) => 2,
                (false, false) => 1,
            };
            let bytes = span(&self.data, cursor, size)?;
            let value = match size {
                4 => i32::from_be_bytes(bytes.try_into().unwrap()),
                2 => i32::from(i16::from_be_bytes(bytes.try_into().unwrap())),
                _ => i32::from(bytes[0] as i8),
            };
            cursor += size;
            let mut scalar = 1.0f64;
            for (&coordinate, &[start, peak, end]) in coordinates.iter().zip(&self.regions[region])
            {
                if peak == 0.0 || coordinate == peak {
                    continue;
                }
                if coordinate <= start || coordinate >= end {
                    scalar = 0.0;
                    break;
                }
                scalar *= if coordinate < peak {
                    f64::from(coordinate - start) / f64::from(peak - start)
                } else {
                    f64::from(end - coordinate) / f64::from(end - peak)
                };
            }
            result += f64::from(value) * scalar;
        }
        let result = result as f32;
        if !result.is_finite() {
            return Err(VariationError::Malformed);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn store(long: bool) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&1u16.to_be_bytes());
        data.extend_from_slice(&12u32.to_be_bytes());
        data.extend_from_slice(&1u16.to_be_bytes());
        data.extend_from_slice(&22u32.to_be_bytes());
        for value in [
            1u16,
            1,
            0,
            16384,
            16384,
            2,
            if long { 0x8001 } else { 1 },
            1,
            0,
        ] {
            data.extend_from_slice(&value.to_be_bytes());
        }
        if long {
            for value in [100i32, -200] {
                data.extend_from_slice(&value.to_be_bytes());
            }
        } else {
            for value in [100i16, -200] {
                data.extend_from_slice(&value.to_be_bytes());
            }
        }
        data
    }
    #[test]
    fn positive_negative_delta_rows_and_normalized_coordinates() {
        for long in [false, true] {
            let parsed = CaretVariationStore::parse(&store(long)).unwrap();
            for (coordinate, expected) in [(0.0, 0.0), (0.25, 25.0), (1.0, 100.0), (-1.0, 0.0)] {
                assert_eq!(parsed.delta(0, 0, &[coordinate]), Ok(expected));
                assert_eq!(parsed.delta(0, 1, &[coordinate]), Ok(-2.0 * expected));
            }
            assert_eq!(
                parsed.delta(1, 0, &[0.0]),
                Err(VariationError::InvalidIndex)
            );
            assert_eq!(
                parsed.delta(0, 2, &[0.0]),
                Err(VariationError::InvalidIndex)
            );
            assert_eq!(parsed.delta(0, 0, &[]), Err(VariationError::AxisMismatch));
            for c in [f32::NAN, f32::INFINITY, 1.1, -1.1] {
                assert_eq!(
                    parsed.delta(0, 0, &[c]),
                    Err(VariationError::InvalidCoordinates)
                );
            }
        }
    }
    #[test]
    fn all_truncated_spans_fail() {
        let data = store(false);
        for end in 0..data.len() {
            assert!(CaretVariationStore::parse(&data[..end]).is_err(), "{end}");
        }
    }
    #[test]
    fn malformed_offsets_counts_regions_fail_closed() {
        for (offset, value) in [
            (0, 2),
            (6, 65535),
            (12, 65535),
            (14, 65535),
            (16, 20000),
            (18, 20000),
            (20, 0),
            (22, 65535),
            (24, 2),
            (26, 65535),
            (28, 1),
        ] {
            let mut data = store(false);
            data[offset..offset + 2].copy_from_slice(&(value as u16).to_be_bytes());
            assert!(
                CaretVariationStore::parse(&data).is_err(),
                "offset {offset}"
            );
        }
        for offset in [2, 8] {
            for value in [0u32, 8, u32::MAX] {
                let mut data = store(false);
                data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
                assert!(CaretVariationStore::parse(&data).is_err());
            }
        }
    }
    #[test]
    fn short_word_and_byte_signed_deltas() {
        // Two identical neutral-axis regions; short and long encodings each
        // combine a leading wide delta with a trailing narrow signed delta.
        for long in [false, true] {
            let mut data = store(long);
            data[14..16].copy_from_slice(&2u16.to_be_bytes());
            data.splice(22..22, [0, 0, 0x40, 0, 0x40, 0]);
            data[8..12].copy_from_slice(&28u32.to_be_bytes());
            data[28..30].copy_from_slice(&1u16.to_be_bytes());
            data[32..34].copy_from_slice(&2u16.to_be_bytes());
            data.splice(36..36, [0, 1]);
            data.truncate(38);
            if long {
                data.extend_from_slice(&100000i32.to_be_bytes());
                data.extend_from_slice(&(-500i16).to_be_bytes());
            } else {
                data.extend_from_slice(&100i16.to_be_bytes());
                data.push((-50i8) as u8);
            }
            let parsed = CaretVariationStore::parse(&data).unwrap();
            assert_eq!(
                parsed.delta(0, 0, &[1.0]),
                Ok(if long { 99500.0 } else { 50.0 })
            );
        }
    }
    #[test]
    fn neutral_axis_peak_zero_and_invalid_region_order() {
        let mut data = store(false);
        for (offset, value) in [(16, -16384i16), (18, 0), (20, 16384)] {
            data[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
        }
        let parsed = CaretVariationStore::parse(&data).unwrap();
        for c in [-1.0, 0.0, 1.0] {
            assert_eq!(parsed.delta(0, 0, &[c]), Ok(100.0));
        }
        data[18..20].copy_from_slice(&8192i16.to_be_bytes()); // nonzero cross-zero peak is unsupported
        assert!(CaretVariationStore::parse(&data).is_err());
        data[16..18].copy_from_slice(&16384i16.to_be_bytes()); // start > peak
        assert!(CaretVariationStore::parse(&data).is_err());
    }

    #[test]
    fn aliased_outer_records_have_bounded_validation_work() {
        let outer_count = 65535usize;
        let header_end = 8 + outer_count * 4;
        let mut data = vec![0; header_end];
        data[0..2].copy_from_slice(&1u16.to_be_bytes());
        data[2..6].copy_from_slice(&(header_end as u32).to_be_bytes());
        data[6..8].copy_from_slice(&(outer_count as u16).to_be_bytes());
        data.extend_from_slice(&[0, 1, 0, 1, 0, 0, 0x40, 0, 0x40, 0]);
        let item_offset = data.len();
        // Repeated region indices make aliases expensive despite tiny input.
        data.extend_from_slice(&[0, 0, 0, 0, 0, 32]);
        data.extend_from_slice(&[0; 64]);
        for i in 0..outer_count {
            data[8 + i * 4..12 + i * 4].copy_from_slice(&(item_offset as u32).to_be_bytes());
        }
        assert_eq!(
            CaretVariationStore::parse(&data).unwrap_err(),
            VariationError::ResourceLimit
        );
    }
}
