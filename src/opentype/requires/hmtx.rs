use bin_rs::reader::BinaryReader;
use std::io::Error;
use std::{fmt, io::SeekFrom};
// hmtx table Font horizontal metrics

#[derive(Debug, Clone)]
pub(crate) struct HMTX {
    pub(crate) h_metrics: Box<Vec<LongHorMetric>>,
}

impl fmt::Display for HMTX {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl HMTX {
    pub(crate) fn new<R: BinaryReader>(
        file: &mut R,
        offest: u32,
        length: u32,
        number_of_hmetrics: u16,
        num_glyphs: u16,
    ) -> Result<Self, Error> {
        get_hmtx(file, offest, length, number_of_hmetrics, num_glyphs)
    }

    pub(crate) fn get_metrix(&self, i: usize) -> LongHorMetric {
        let h_metric = self.h_metrics.get(i);
        match h_metric {
            Some(h_metric) => h_metric.clone(),
            None => LongHorMetric {
                advance_width: 0,
                left_side_bearing: 0,
            },
        }
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = "hmtx\n".to_string();
        let max_len = 10;
        for (i, h_metric) in self.h_metrics.iter().enumerate() {
            let advance_width = format!("{i} Advance Width {} ", h_metric.advance_width);
            string += &advance_width;
            let left_side_bearing = format!("Left Side Bearing {}\n", h_metric.left_side_bearing);
            string += &left_side_bearing;
            if max_len < i {
                break;
            }
        }
        string
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LongHorMetric {
    pub(crate) advance_width: u16,
    pub(crate) left_side_bearing: i16,
}

fn get_hmtx<R: bin_rs::reader::BinaryReader>(
    file: &mut R,
    offest: u32,
    _length: u32,
    number_of_hmetrics: u16,
    num_glyphs: u16,
) -> Result<HMTX, Error> {
    let file = file;
    file.seek(SeekFrom::Start(offest as u64))?;
    let mut h_metrics = Vec::new();
    for _ in 0..number_of_hmetrics {
        let advance_width = file.read_u16_be()?;
        let left_side_bearing = file.read_i16_be()?;
        h_metrics.push(LongHorMetric {
            advance_width,
            left_side_bearing,
        });
    }
    let advance_width = h_metrics
        .last()
        .map(|metric| metric.advance_width)
        .unwrap_or(0);
    let number = num_glyphs.saturating_sub(number_of_hmetrics);
    for _ in 0..number {
        let left_side_bearing = file.read_i16_be()?;
        h_metrics.push(LongHorMetric {
            advance_width,
            left_side_bearing,
        });
    }
    Ok(HMTX {
        h_metrics: Box::new(h_metrics),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bin_rs::reader::BytesReader;

    #[test]
    fn hmtx_allows_zero_long_metrics() {
        let buffer = [0x00, 0x01, 0xFF, 0xFE, 0x00, 0x03];
        let mut reader = BytesReader::new(&buffer);
        let hmtx = HMTX::new(&mut reader, 0, buffer.len() as u32, 0, 3).expect("parse hmtx");

        assert_eq!(hmtx.h_metrics.len(), 3);
        assert_eq!(hmtx.get_metrix(0).advance_width, 0);
        assert_eq!(hmtx.get_metrix(0).left_side_bearing, 1);
        assert_eq!(hmtx.get_metrix(1).left_side_bearing, -2);
        assert_eq!(hmtx.get_metrix(2).left_side_bearing, 3);
        assert_eq!(hmtx.get_metrix(99).advance_width, 0);
    }
}
