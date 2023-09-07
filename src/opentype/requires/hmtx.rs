use bin_rs::reader::BinaryReader;
use std::{fmt, io::SeekFrom};

// hmtx table Font horizontal metrics

#[derive(Debug, Clone)]
pub(crate) struct HMTX {
    pub(crate) h_metrics: Box<Vec<LongHorMetric>>,
    pub(crate) left_side_bearings: Box<Vec<u16>>,
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
    ) -> Self {
        get_hdmx(file, offest, length, number_of_hmetrics, num_glyphs)
    }

    pub(crate) fn get_metrix(&self, i: usize) -> LongHorMetric {
        let h_metric = self.h_metrics.get(i).unwrap();
        h_metric.clone()
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
        for (i, left_side_bearing) in self.left_side_bearings.iter().enumerate() {
            let left_side_bearing = format!("{i} Left Side Bearing {}\n", left_side_bearing);
            string += &left_side_bearing;
            if max_len < i {
                break;
            }
        }
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LongHorMetric {
    pub(crate) advance_width: u16,
    pub(crate) left_side_bearing: i16,
}

fn get_hdmx<R: bin_rs::reader::BinaryReader>(
    file: &mut R,
    offest: u32,
    _length: u32,
    number_of_hmetrics: u16,
    num_glyphs: u16,
) -> HMTX {
    let file = file;
    file.seek(SeekFrom::Start(offest as u64)).unwrap();
    let mut h_metrics = Vec::new();
    for _ in 0..number_of_hmetrics {
        let advance_width = file.read_u16_be().unwrap();
        let left_side_bearing = file.read_i16_be().unwrap();
        h_metrics.push(LongHorMetric {
            advance_width,
            left_side_bearing,
        });
    }
    let mut left_side_bearings = Vec::new();
    let number = num_glyphs - number_of_hmetrics;
    for _ in 0..number {
        let left_side_bearing = file.read_u16_be().unwrap();
        left_side_bearings.push(left_side_bearing);
    }
    HMTX {
        h_metrics: Box::new(h_metrics),
        left_side_bearings: Box::new(left_side_bearings),
    }
}
