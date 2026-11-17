//! Bounded, unhinted real-point lookup for GDEF CaretValue format 2.
//! Returns raw glyf coordinates, not metric-origin-adjusted coordinates.
//! Composite outlines, implied curve midpoints and phantom points are excluded.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaretPointError {
    Malformed,
    CompositeUnsupported,
    PointOutOfRange,
    CoordinateUnsafe,
}

fn byte(data: &[u8], cursor: &mut usize) -> Result<u8, CaretPointError> {
    let value = *data.get(*cursor).ok_or(CaretPointError::Malformed)?;
    *cursor += 1;
    Ok(value)
}

fn word(data: &[u8], cursor: &mut usize) -> Result<u16, CaretPointError> {
    Ok(u16::from_be_bytes([
        byte(data, cursor)?,
        byte(data, cursor)?,
    ]))
}

fn delta(
    data: &[u8],
    cursor: &mut usize,
    flag: u8,
    short: u8,
    same: u8,
) -> Result<i32, CaretPointError> {
    if flag & short != 0 {
        let value = i32::from(byte(data, cursor)?);
        Ok(if flag & same != 0 { value } else { -value })
    } else if flag & same != 0 {
        Ok(0)
    } else {
        Ok(i32::from(word(data, cursor)? as i16))
    }
}

/// Decode the entire simple-glyph stream before returning a real point.
/// `data` must be restricted to this glyph's declared loca span. Padding after
/// the coordinate stream is allowed. The caller owns static-font selection,
/// metric-origin conversion, axis selection, scaling and hinting policy.
#[cfg(test)] // Raw mathematical decoder retained for independent unit controls.
pub(crate) fn simple_glyph_point(
    data: &[u8],
    point_index: u16,
) -> Result<(i32, i32), CaretPointError> {
    decode_point(data, point_index, None)
}

/// Conservative eligibility check for the legacy i16 renderer, not whole-font
/// validation. Checks every real point, including points other than the caret,
/// and adjacent/closing midpoint sums in each contour. No coordinate is changed.
/// The caller must supply the exact, checked renderer baseline before narrowing.
#[cfg_attr(not(feature = "layout"), allow(dead_code))]
pub(crate) fn simple_glyph_point_for_render(
    data: &[u8],
    point_index: u16,
    baseline: i32,
) -> Result<(i32, i32), CaretPointError> {
    if i16::try_from(baseline).is_err() {
        return Err(CaretPointError::CoordinateUnsafe);
    }
    decode_point(data, point_index, Some(baseline))
}

fn decode_point(
    data: &[u8],
    point_index: u16,
    baseline: Option<i32>,
) -> Result<(i32, i32), CaretPointError> {
    if data.is_empty() {
        return Err(CaretPointError::PointOutOfRange);
    }
    if data.len() < 10 {
        return Err(CaretPointError::Malformed);
    }
    let contours = i16::from_be_bytes([data[0], data[1]]);
    if contours < 0 {
        return Err(CaretPointError::CompositeUnsupported);
    }
    let mut cursor = 10;
    let mut previous = None;
    let mut endpoints = Vec::with_capacity(contours as usize);
    for _ in 0..contours {
        let end = word(data, &mut cursor)?;
        if previous.is_some_and(|prior| end <= prior) {
            return Err(CaretPointError::Malformed);
        }
        previous = Some(end);
        endpoints.push(usize::from(end));
    }
    // Empty outlines may consist of the header alone; there is no point stream.
    if contours == 0 && cursor == data.len() {
        return Err(CaretPointError::PointOutOfRange);
    }
    let instructions = usize::from(word(data, &mut cursor)?);
    cursor = cursor
        .checked_add(instructions)
        .ok_or(CaretPointError::Malformed)?;
    if cursor > data.len() {
        return Err(CaretPointError::Malformed);
    }
    let count = previous.map_or(0, |end| usize::from(end) + 1);
    let mut flags = Vec::with_capacity(count);
    while flags.len() < count {
        let flag = byte(data, &mut cursor)?;
        if flag & 0x80 != 0 {
            return Err(CaretPointError::Malformed);
        }
        let repeat = if flag & 8 != 0 {
            usize::from(byte(data, &mut cursor)?)
        } else {
            0
        };
        if repeat + 1 > count - flags.len() {
            return Err(CaretPointError::Malformed);
        }
        flags.resize(flags.len() + repeat + 1, flag);
    }
    let index = usize::from(point_index);
    let mut point = (0, 0);
    let mut points = baseline.map(|_| vec![(0i32, 0i32); count]);
    let mut x = 0i32;
    for (i, flag) in flags.iter().enumerate() {
        x = x
            .checked_add(delta(data, &mut cursor, *flag, 2, 16)?)
            .ok_or(CaretPointError::Malformed)?;
        if let Some(points) = points.as_mut() {
            points[i].0 = x;
        }
        if i == index {
            point.0 = x;
        }
    }
    let mut y = 0i32;
    for (i, flag) in flags.iter().enumerate() {
        y = y
            .checked_add(delta(data, &mut cursor, *flag, 4, 32)?)
            .ok_or(CaretPointError::Malformed)?;
        if let Some(points) = points.as_mut() {
            points[i].1 = y;
        }
        if i == index {
            point.1 = y;
        }
    }
    if let (Some(baseline), Some(points)) = (baseline, points) {
        for &(x, y) in &points {
            if i16::try_from(x).is_err()
                || i16::try_from(y).is_err()
                || baseline
                    .checked_sub(y)
                    .and_then(|v| i16::try_from(v).ok())
                    .is_none()
            {
                return Err(CaretPointError::CoordinateUnsafe);
            }
        }
        let mut start = 0;
        for end in endpoints {
            for index in start..=end {
                let next = if index == end { start } else { index + 1 };
                let a = points[index];
                let b = points[next];
                // Even on-curve pairs are checked conservatively; eligibility
                // must not depend on which midpoint path the renderer chooses.
                if i16::try_from(a.0 + b.0).is_err() || i16::try_from(a.1 + b.1).is_err() {
                    return Err(CaretPointError::CoordinateUnsafe);
                }
            }
            start = end + 1;
        }
    }
    if index >= count {
        return Err(CaretPointError::PointOutOfRange);
    }
    Ok(point)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(contours: i16) -> Vec<u8> {
        let mut data = vec![0; 10];
        data[..2].copy_from_slice(&contours.to_be_bytes());
        data
    }

    fn sample() -> Vec<u8> {
        let mut data = header(2);
        data.extend_from_slice(&[0, 1, 0, 2, 0, 1, 0xaa]); // endpoints; one instruction
        data.extend_from_slice(&[1, 0, 1]); // middle point is off curve
        for v in [100i16, 200, -50, -100, 120, 30] {
            data.extend_from_slice(&v.to_be_bytes());
        }
        data
    }

    #[test]
    fn actual_points_keep_off_curve_and_cross_contour_deltas() {
        let data = sample();
        assert_eq!(simple_glyph_point(&data, 0), Ok((100, -100)));
        assert_eq!(simple_glyph_point(&data, 1), Ok((300, 20)));
        assert_eq!(simple_glyph_point(&data, 2), Ok((250, 50)));
        for i in 3..8 {
            assert_eq!(
                simple_glyph_point(&data, i),
                Err(CaretPointError::PointOutOfRange)
            );
        }
    }

    #[test]
    fn truncations_rejected_even_after_requested_point() {
        let data = sample();
        for end in 1..data.len() {
            assert_eq!(
                simple_glyph_point(&data[..end], 0),
                Err(CaretPointError::Malformed),
                "length {end}"
            );
        }
        let mut padded = data.clone();
        padded.extend_from_slice(&[0, 0, 0]);
        assert_eq!(simple_glyph_point(&padded, 0), Ok((100, -100)));
    }

    #[test]
    fn repeat_short_and_zero_deltas() {
        let mut data = header(1);
        data.extend_from_slice(&[0, 2, 0, 0, 0x3f, 1, 0x31, 10, 20, 30, 40]);
        assert_eq!(simple_glyph_point(&data, 2), Ok((30, 70)));
        data[14] = 0x0f; // short negative both axes, repeated twice
        assert_eq!(simple_glyph_point(&data, 2), Ok((-30, -70)));
        data[15] = 3; // repeat exceeds declared real-point count
        assert_eq!(
            simple_glyph_point(&data, 0),
            Err(CaretPointError::Malformed)
        );
    }

    #[test]
    fn invalid_endpoints_instructions_and_reserved_flag() {
        let mut data = sample();
        data[13] = 1; // duplicate endpoint
        assert_eq!(
            simple_glyph_point(&data, 0),
            Err(CaretPointError::Malformed)
        );
        data[13] = 0; // decreasing endpoint
        assert_eq!(
            simple_glyph_point(&data, 0),
            Err(CaretPointError::Malformed)
        );
        let mut data = sample();
        data[14] = 0xff;
        assert_eq!(
            simple_glyph_point(&data, 0),
            Err(CaretPointError::Malformed)
        );
        let mut data = sample();
        data[17] |= 0x80;
        assert_eq!(
            simple_glyph_point(&data, 0),
            Err(CaretPointError::Malformed)
        );
    }

    #[test]
    fn empty_and_composite_are_not_points() {
        assert_eq!(
            simple_glyph_point(&[], 0),
            Err(CaretPointError::PointOutOfRange)
        );
        assert_eq!(
            simple_glyph_point(&header(0), 0),
            Err(CaretPointError::PointOutOfRange)
        );
        let mut empty = header(0);
        empty.extend_from_slice(&[0, 0]);
        assert_eq!(
            simple_glyph_point(&empty, 0),
            Err(CaretPointError::PointOutOfRange)
        );
        assert_eq!(
            simple_glyph_point(&header(-1), 0),
            Err(CaretPointError::CompositeUnsupported)
        );
    }

    #[test]
    fn accumulation_does_not_wrap_i16() {
        let mut data = header(1);
        data.extend_from_slice(&[0, 1, 0, 0, 0x21, 0x21, 0x7f, 0xff, 0x7f, 0xff]);
        assert_eq!(simple_glyph_point(&data, 1), Ok((65534, 0)));
    }

    fn points_fixture(points: &[(i16, i16)], endpoints: &[u16]) -> Vec<u8> {
        let mut data = header(endpoints.len() as i16);
        for end in endpoints {
            data.extend_from_slice(&end.to_be_bytes());
        }
        data.extend_from_slice(&[0, 0]);
        data.extend(std::iter::repeat_n(0u8, points.len())); // all off-curve
        for axis in 0..2 {
            let mut previous = 0i32;
            for &(x, y) in points {
                let value = i32::from(if axis == 0 { x } else { y });
                let delta = i16::try_from(value - previous).unwrap();
                data.extend_from_slice(&delta.to_be_bytes());
                previous = value;
            }
        }
        data
    }

    #[test]
    fn render_guard_checks_selected_and_unselected_accumulation() {
        let mut data = header(1);
        data.extend_from_slice(&[0, 1, 0, 0, 0x21, 0x21, 0x7f, 0xff, 0x7f, 0xff]);
        for selected in [0, 1] {
            assert_eq!(
                simple_glyph_point_for_render(&data, selected, 0),
                Err(CaretPointError::CoordinateUnsafe)
            );
        }
        assert_eq!(simple_glyph_point(&data, 1), Ok((65534, 0)));
    }

    #[test]
    fn render_guard_checks_adjacent_and_closing_midpoint_sums() {
        for points in [
            vec![(16000, 0), (17000, 0), (1000, 0)],
            vec![(17000, 0), (0, 0), (17000, 0)], // only closing pair overflows
            vec![(0, -17000), (0, -16000), (0, -1000)],
        ] {
            let data = points_fixture(&points, &[2]);
            assert_eq!(
                simple_glyph_point_for_render(&data, 1, 0),
                Err(CaretPointError::CoordinateUnsafe)
            );
        }
    }

    #[test]
    fn render_guard_checks_baseline_and_every_y_difference() {
        let data = points_fixture(&[(0, 0), (0, -100)], &[1]);
        for baseline in [32768, -32769, i32::MAX, i32::MIN, 32767] {
            assert_eq!(
                simple_glyph_point_for_render(&data, 0, baseline),
                Err(CaretPointError::CoordinateUnsafe)
            );
        }
        let data = points_fixture(&[(0, 0), (0, 100)], &[1]);
        assert_eq!(
            simple_glyph_point_for_render(&data, 0, -32768),
            Err(CaretPointError::CoordinateUnsafe)
        );
    }

    #[test]
    fn render_guard_preserves_raw_point_and_respects_contour_boundaries() {
        let data = sample();
        assert_eq!(simple_glyph_point_for_render(&data, 1, 800), Ok((300, 20)));
        let data = points_fixture(&[(17000, 0), (0, 0), (0, 0), (17000, 0)], &[1, 3]);
        assert_eq!(simple_glyph_point_for_render(&data, 3, 0), Ok((17000, 0)));
        // Across-contour last/first sums are deliberately not midpoint pairs.
    }
}
