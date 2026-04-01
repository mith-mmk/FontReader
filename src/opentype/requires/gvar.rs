use std::io::{Error, ErrorKind};

use crate::opentype::outline::glyf::ParsedGlyph;

#[derive(Debug, Clone)]
pub(crate) struct GVAR {
    data: Vec<u8>,
    axis_count: u16,
    shared_tuples: Vec<f32>,
    offsets: GlyphVariationDataOffsets,
    glyph_variation_data_offset: usize,
}

#[derive(Debug, Clone)]
enum GlyphVariationDataOffsets {
    Short(Vec<u16>),
    Long(Vec<u32>),
}

#[derive(Debug, Clone)]
struct TupleVariation {
    point_numbers: Option<Vec<u16>>,
    deltas: Vec<(f32, f32)>,
}

#[derive(Debug, Clone, Copy)]
struct TupleVariationHeader {
    scalar: f32,
    has_private_point_numbers: bool,
    serialized_data_len: u16,
}

impl GVAR {
    pub(crate) fn new<R: bin_rs::reader::BinaryReader>(
        reader: &mut R,
        offset: u32,
        length: u32,
    ) -> Result<Self, Error> {
        use std::io::SeekFrom;

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
                format!("unsupported gvar version: {version:#010x}"),
            ));
        }

        let axis_count = read_u16(data, &mut cursor)
            .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "unexpected end of gvar data"))?;
        if axis_count == 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "gvar axis_count must be greater than zero",
            ));
        }

        let shared_tuple_count = read_u16(data, &mut cursor)
            .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "unexpected end of gvar data"))?
            as usize;
        let shared_tuples_offset = read_u32(data, &mut cursor)? as usize;
        let glyph_count = read_u16(data, &mut cursor)
            .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "unexpected end of gvar data"))?
            as usize;
        let flags = read_u16(data, &mut cursor)
            .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "unexpected end of gvar data"))?;
        let glyph_variation_data_offset = read_u32(data, &mut cursor)? as usize;

        let offsets_count = glyph_count
            .checked_add(1)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "gvar glyph offset overflow"))?;
        let offsets = if (flags & 1) != 0 {
            let mut values = Vec::with_capacity(offsets_count);
            for _ in 0..offsets_count {
                values.push(read_u32(data, &mut cursor)?);
            }
            GlyphVariationDataOffsets::Long(values)
        } else {
            let mut values = Vec::with_capacity(offsets_count);
            for _ in 0..offsets_count {
                values.push(read_u16(data, &mut cursor).ok_or_else(|| {
                    Error::new(ErrorKind::UnexpectedEof, "unexpected end of gvar data")
                })?);
            }
            GlyphVariationDataOffsets::Short(values)
        };

        let shared_tuple_len = shared_tuple_count
            .checked_mul(axis_count as usize)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "gvar shared tuple overflow"))?;
        let mut shared_cursor = shared_tuples_offset;
        let mut shared_tuples = Vec::with_capacity(shared_tuple_len);
        for _ in 0..shared_tuple_len {
            shared_tuples.push(f2dot14_to_f32(
                read_i16(data, &mut shared_cursor).ok_or_else(|| {
                    Error::new(ErrorKind::UnexpectedEof, "unexpected end of gvar data")
                })?,
            ));
        }

        Ok(Self {
            data: data.to_vec(),
            axis_count,
            shared_tuples,
            offsets,
            glyph_variation_data_offset,
        })
    }

    pub(crate) fn apply_to_parsed_glyph(
        &self,
        glyph_id: usize,
        coordinates: &[f32],
        parsed: &ParsedGlyph,
    ) -> Option<ParsedGlyph> {
        if coordinates.is_empty()
            || parsed.number_of_contours <= 0
            || coordinates.len() != self.axis_count as usize
            || parsed.xs.is_empty()
        {
            return None;
        }

        let variation_data = self.glyph_variation_data(glyph_id)?;
        if variation_data.is_empty() {
            return None;
        }

        let point_count = parsed.xs.len();
        let total_points = point_count.checked_add(4)? as u16;
        let tuples = self.parse_glyph_variation_data(variation_data, coordinates, total_points)?;
        if tuples.is_empty() {
            return None;
        }

        let base_points = absolute_points(parsed);
        if base_points.is_empty() {
            return None;
        }
        let contour_ranges = contour_ranges(parsed);
        let mut varied_points = base_points
            .iter()
            .map(|&(x, y)| (x as f32, y as f32))
            .collect::<Vec<_>>();

        let mut any_change = false;
        for tuple in &tuples {
            any_change |= apply_tuple_to_points(
                &mut varied_points,
                &base_points,
                &contour_ranges,
                point_count,
                tuple,
            );
        }

        if !any_change {
            return None;
        }

        Some(rebuild_parsed_glyph(parsed, &varied_points))
    }

    fn glyph_variation_data(&self, glyph_id: usize) -> Option<&[u8]> {
        let (start, end) = match &self.offsets {
            GlyphVariationDataOffsets::Short(offsets) => {
                let start = *offsets.get(glyph_id)? as usize * 2;
                let end = *offsets.get(glyph_id.checked_add(1)?)? as usize * 2;
                (start, end)
            }
            GlyphVariationDataOffsets::Long(offsets) => {
                let start = *offsets.get(glyph_id)? as usize;
                let end = *offsets.get(glyph_id.checked_add(1)?)? as usize;
                (start, end)
            }
        };

        if start == end {
            return None;
        }

        let base = self.glyph_variation_data_offset;
        self.data
            .get(base.checked_add(start)?..base.checked_add(end)?)
    }

    fn parse_glyph_variation_data(
        &self,
        data: &[u8],
        coordinates: &[f32],
        total_points: u16,
    ) -> Option<Vec<TupleVariation>> {
        const SHARED_POINT_NUMBERS_FLAG: u16 = 0x8000;
        const COUNT_MASK: u16 = 0x0FFF;

        let mut header_cursor = 0usize;
        let tuple_variation_count = read_u16(data, &mut header_cursor)?;
        let serialized_offset = read_u16(data, &mut header_cursor)? as usize;
        let has_shared_point_numbers = (tuple_variation_count & SHARED_POINT_NUMBERS_FLAG) != 0;
        let tuple_count = (tuple_variation_count & COUNT_MASK) as usize;
        if tuple_count == 0 {
            return None;
        }

        let mut serialized_cursor = serialized_offset;
        let shared_points = if has_shared_point_numbers {
            let (points, consumed) = parse_packed_points(data.get(serialized_cursor..)?)?;
            serialized_cursor = serialized_cursor.checked_add(consumed)?;
            points
        } else {
            None
        };

        let mut tuples = Vec::new();
        for _ in 0..tuple_count {
            let header =
                self.parse_tuple_variation_header(data, &mut header_cursor, coordinates)?;
            let serialized_end =
                serialized_cursor.checked_add(header.serialized_data_len as usize)?;
            let tuple_bytes = data.get(serialized_cursor..serialized_end)?;
            serialized_cursor = serialized_end;

            if header.scalar <= 0.0 {
                continue;
            }

            let (point_numbers, points_consumed) = if header.has_private_point_numbers {
                let (points, consumed) = parse_packed_points(tuple_bytes)?;
                (points, consumed)
            } else {
                (shared_points.clone(), 0usize)
            };

            let delta_count = point_numbers
                .as_ref()
                .map(|points| points.len())
                .unwrap_or(total_points as usize);
            let deltas = parse_packed_deltas(
                tuple_bytes.get(points_consumed..)?,
                delta_count,
                header.scalar,
            )?;
            tuples.push(TupleVariation {
                point_numbers,
                deltas,
            });
        }

        Some(tuples)
    }

    fn parse_tuple_variation_header(
        &self,
        data: &[u8],
        cursor: &mut usize,
        coordinates: &[f32],
    ) -> Option<TupleVariationHeader> {
        const EMBEDDED_PEAK_TUPLE_FLAG: u16 = 0x8000;
        const INTERMEDIATE_REGION_FLAG: u16 = 0x4000;
        const PRIVATE_POINT_NUMBERS_FLAG: u16 = 0x2000;
        const TUPLE_INDEX_MASK: u16 = 0x0FFF;

        let serialized_data_len = read_u16(data, cursor)?;
        let tuple_index = read_u16(data, cursor)?;
        let has_embedded_peak_tuple = (tuple_index & EMBEDDED_PEAK_TUPLE_FLAG) != 0;
        let has_intermediate_region = (tuple_index & INTERMEDIATE_REGION_FLAG) != 0;
        let has_private_point_numbers = (tuple_index & PRIVATE_POINT_NUMBERS_FLAG) != 0;
        let tuple_index = (tuple_index & TUPLE_INDEX_MASK) as usize;
        let axis_count = self.axis_count as usize;

        let peak_tuple = if has_embedded_peak_tuple {
            let mut values = Vec::with_capacity(axis_count);
            for _ in 0..axis_count {
                values.push(f2dot14_to_f32(read_i16(data, cursor)?));
            }
            values
        } else {
            let start = tuple_index.checked_mul(axis_count)?;
            let end = start.checked_add(axis_count)?;
            self.shared_tuples.get(start..end)?.to_vec()
        };

        let (start_tuple, end_tuple) = if has_intermediate_region {
            let mut start_values = Vec::with_capacity(axis_count);
            let mut end_values = Vec::with_capacity(axis_count);
            for _ in 0..axis_count {
                start_values.push(f2dot14_to_f32(read_i16(data, cursor)?));
            }
            for _ in 0..axis_count {
                end_values.push(f2dot14_to_f32(read_i16(data, cursor)?));
            }
            (Some(start_values), Some(end_values))
        } else {
            (None, None)
        };

        let scalar = compute_tuple_scalar(
            coordinates,
            &peak_tuple,
            start_tuple.as_deref(),
            end_tuple.as_deref(),
        );
        Some(TupleVariationHeader {
            scalar,
            has_private_point_numbers,
            serialized_data_len,
        })
    }
}

fn compute_tuple_scalar(
    coordinates: &[f32],
    peak_tuple: &[f32],
    start_tuple: Option<&[f32]>,
    end_tuple: Option<&[f32]>,
) -> f32 {
    let mut scalar = 1.0f32;
    for axis in 0..coordinates.len() {
        let value = coordinates[axis];
        let peak = *peak_tuple.get(axis).unwrap_or(&0.0);
        if peak == 0.0 || value == peak {
            continue;
        }

        if let (Some(start_tuple), Some(end_tuple)) = (start_tuple, end_tuple) {
            let start = *start_tuple.get(axis).unwrap_or(&0.0);
            let end = *end_tuple.get(axis).unwrap_or(&0.0);
            if start > peak || peak > end || (start < 0.0 && end > 0.0 && peak != 0.0) {
                continue;
            }
            if value < start || value > end {
                return 0.0;
            }
            if value < peak {
                if peak != start {
                    scalar *= (value - start) / (peak - start);
                }
            } else if peak != end {
                scalar *= (end - value) / (end - peak);
            }
        } else if value == 0.0 || value < 0.0f32.min(peak) || value > 0.0f32.max(peak) {
            return 0.0;
        } else {
            scalar *= value / peak;
        }
    }

    scalar
}

fn parse_packed_points(data: &[u8]) -> Option<(Option<Vec<u16>>, usize)> {
    let mut cursor = 0usize;
    let first = read_u8(data, &mut cursor)?;
    let mut count = first as u16;
    if (first & 0x80) != 0 {
        count = (((first & 0x7F) as u16) << 8) | read_u8(data, &mut cursor)? as u16;
    }
    if count == 0 {
        return Some((None, cursor));
    }

    let mut points = Vec::with_capacity(count as usize);
    let mut current = 0u16;
    while points.len() < count as usize {
        let control = read_u8(data, &mut cursor)?;
        let is_words = (control & 0x80) != 0;
        let run_count = (control & 0x7F) as usize + 1;
        for _ in 0..run_count {
            let delta = if is_words {
                read_u16(data, &mut cursor)?
            } else {
                read_u8(data, &mut cursor)? as u16
            };
            current = current.checked_add(delta)?;
            points.push(current);
            if points.len() == count as usize {
                break;
            }
        }
    }

    Some((Some(points), cursor))
}

fn parse_packed_deltas(data: &[u8], count: usize, scalar: f32) -> Option<Vec<(f32, f32)>> {
    let (x_deltas, cursor) = parse_axis_deltas(data, 0, count, scalar)?;
    let (y_deltas, _) = parse_axis_deltas(data, cursor, count, scalar)?;
    Some(x_deltas.into_iter().zip(y_deltas).collect())
}

fn parse_axis_deltas(
    data: &[u8],
    mut cursor: usize,
    count: usize,
    scalar: f32,
) -> Option<(Vec<f32>, usize)> {
    let mut deltas = Vec::with_capacity(count);
    while deltas.len() < count {
        let control = read_u8_at(data, cursor)?;
        cursor += 1;
        let deltas_are_zero = (control & 0x80) != 0;
        let deltas_are_words = (control & 0x40) != 0;
        let run_count = (control & 0x3F) as usize + 1;

        if deltas_are_zero {
            for _ in 0..run_count {
                deltas.push(0.0);
                if deltas.len() == count {
                    break;
                }
            }
            continue;
        }

        for _ in 0..run_count {
            let value = if deltas_are_words {
                read_i16(data, &mut cursor)? as f32
            } else {
                read_i8(data, &mut cursor)? as f32
            };
            deltas.push(value * scalar);
            if deltas.len() == count {
                break;
            }
        }
    }

    Some((deltas, cursor))
}

fn apply_tuple_to_points(
    points: &mut [(f32, f32)],
    base_points: &[(i16, i16)],
    contour_ranges: &[(usize, usize)],
    point_count: usize,
    tuple: &TupleVariation,
) -> bool {
    let mut changed = false;
    if let Some(point_numbers) = tuple.point_numbers.as_ref() {
        let mut explicit = vec![None::<(f32, f32)>; point_count];
        for (point_index, delta) in point_numbers
            .iter()
            .copied()
            .zip(tuple.deltas.iter().copied())
        {
            let point_index = point_index as usize;
            if point_index >= point_count {
                continue;
            }
            match &mut explicit[point_index] {
                Some(existing) => {
                    existing.0 += delta.0;
                    existing.1 += delta.1;
                }
                None => {
                    explicit[point_index] = Some(delta);
                }
            }
        }

        for &(start, end) in contour_ranges {
            let explicit_indices = (start..=end)
                .filter(|&index| explicit[index].is_some())
                .collect::<Vec<_>>();
            if explicit_indices.is_empty() {
                continue;
            }

            for index in start..=end {
                let (dx, dy) = if let Some(delta) = explicit[index] {
                    delta
                } else {
                    infer_contour_delta(base_points, &explicit, &explicit_indices, index)
                };
                if dx != 0.0 || dy != 0.0 {
                    points[index].0 += dx;
                    points[index].1 += dy;
                    changed = true;
                }
            }
        }
    } else {
        for (index, delta) in tuple.deltas.iter().copied().enumerate().take(point_count) {
            if delta.0 != 0.0 || delta.1 != 0.0 {
                points[index].0 += delta.0;
                points[index].1 += delta.1;
                changed = true;
            }
        }
    }

    changed
}

fn infer_contour_delta(
    base_points: &[(i16, i16)],
    explicit: &[Option<(f32, f32)>],
    explicit_indices: &[usize],
    target_index: usize,
) -> (f32, f32) {
    let mut next_position = 0usize;
    while next_position < explicit_indices.len() && explicit_indices[next_position] < target_index {
        next_position += 1;
    }

    let prev_index = if next_position == 0 {
        explicit_indices[explicit_indices.len() - 1]
    } else {
        explicit_indices[next_position - 1]
    };
    let next_index = if next_position == explicit_indices.len() {
        explicit_indices[0]
    } else {
        explicit_indices[next_position]
    };

    let (prev_dx, prev_dy) = explicit[prev_index].unwrap_or((0.0, 0.0));
    let (next_dx, next_dy) = explicit[next_index].unwrap_or((0.0, 0.0));
    let prev = base_points[prev_index];
    let target = base_points[target_index];
    let next = base_points[next_index];

    (
        infer_delta(prev.0, target.0, next.0, prev_dx, next_dx),
        infer_delta(prev.1, target.1, next.1, prev_dy, next_dy),
    )
}

fn infer_delta(
    prev_point: i16,
    target_point: i16,
    next_point: i16,
    prev_delta: f32,
    next_delta: f32,
) -> f32 {
    if prev_point == next_point {
        if prev_delta == next_delta {
            prev_delta
        } else {
            0.0
        }
    } else if target_point <= prev_point.min(next_point) {
        if prev_point < next_point {
            prev_delta
        } else {
            next_delta
        }
    } else if target_point >= prev_point.max(next_point) {
        if prev_point > next_point {
            prev_delta
        } else {
            next_delta
        }
    } else {
        let distance = (target_point - prev_point) as f32 / (next_point - prev_point) as f32;
        (1.0 - distance) * prev_delta + distance * next_delta
    }
}

fn absolute_points(parsed: &ParsedGlyph) -> Vec<(i16, i16)> {
    let mut points = Vec::with_capacity(parsed.xs.len());
    let mut x = 0i16;
    let mut y = 0i16;
    for (&dx, &dy) in parsed.xs.iter().zip(parsed.ys.iter()) {
        x = x.wrapping_add(dx);
        y = y.wrapping_add(dy);
        points.push((x, y));
    }
    points
}

fn contour_ranges(parsed: &ParsedGlyph) -> Vec<(usize, usize)> {
    let mut ranges = Vec::with_capacity(parsed.end_pts_of_contours.len());
    let mut start = 0usize;
    for &end in &parsed.end_pts_of_contours {
        ranges.push((start, end));
        start = end.saturating_add(1);
    }
    ranges
}

fn rebuild_parsed_glyph(parsed: &ParsedGlyph, points: &[(f32, f32)]) -> ParsedGlyph {
    let rounded = points
        .iter()
        .map(|&(x, y)| (round_to_i16(x), round_to_i16(y)))
        .collect::<Vec<_>>();

    let mut xs = Vec::with_capacity(rounded.len());
    let mut ys = Vec::with_capacity(rounded.len());
    let mut prev_x = 0i16;
    let mut prev_y = 0i16;
    let mut x_min = i16::MAX;
    let mut y_min = i16::MAX;
    let mut x_max = i16::MIN;
    let mut y_max = i16::MIN;

    for &(x, y) in &rounded {
        xs.push(x.wrapping_sub(prev_x));
        ys.push(y.wrapping_sub(prev_y));
        prev_x = x;
        prev_y = y;
        x_min = x_min.min(x);
        y_min = y_min.min(y);
        x_max = x_max.max(x);
        y_max = y_max.max(y);
    }

    if rounded.is_empty() {
        x_min = 0;
        y_min = 0;
        x_max = 0;
        y_max = 0;
    }

    ParsedGlyph {
        number_of_contours: parsed.number_of_contours,
        x_min,
        y_min,
        x_max,
        y_max,
        offset: parsed.offset,
        length: parsed.length,
        end_pts_of_contours: parsed.end_pts_of_contours.clone(),
        instructions: parsed.instructions.clone(),
        flags: parsed.flags.clone(),
        xs,
        ys,
        on_curves: parsed.on_curves.clone(),
    }
}

fn round_to_i16(value: f32) -> i16 {
    value.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

fn read_u8(data: &[u8], cursor: &mut usize) -> Option<u8> {
    let value = *data.get(*cursor)?;
    *cursor += 1;
    Some(value)
}

fn read_u8_at(data: &[u8], cursor: usize) -> Option<u8> {
    data.get(cursor).copied()
}

fn read_i8(data: &[u8], cursor: &mut usize) -> Option<i8> {
    Some(read_u8(data, cursor)? as i8)
}

fn read_u16(data: &[u8], cursor: &mut usize) -> Option<u16> {
    let bytes = read_bytes::<2>(data, cursor)?;
    Some(u16::from_be_bytes(bytes))
}

fn read_i16(data: &[u8], cursor: &mut usize) -> Option<i16> {
    let bytes = read_bytes::<2>(data, cursor)?;
    Some(i16::from_be_bytes(bytes))
}

fn read_u32(data: &[u8], cursor: &mut usize) -> Result<u32, Error> {
    let bytes = read_bytes::<4>(data, cursor)
        .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "unexpected end of gvar data"))?;
    Ok(u32::from_be_bytes(bytes))
}

fn read_bytes<const N: usize>(data: &[u8], cursor: &mut usize) -> Option<[u8; N]> {
    let end = cursor.checked_add(N)?;
    let slice = data.get(*cursor..end)?;
    let mut bytes = [0u8; N];
    bytes.copy_from_slice(slice);
    *cursor = end;
    Some(bytes)
}

fn f2dot14_to_f32(value: i16) -> f32 {
    value as f32 / 16384.0
}
