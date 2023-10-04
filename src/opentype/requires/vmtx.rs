use bin_rs::reader::BinaryReader;
use std::{fmt, io::SeekFrom};

// vmtx table Font vertical metrics

#[derive(Debug, Clone)]
pub(crate) struct VMTX {
    pub(crate) v_metrics: Box<Vec<VerticalMetric>>,
    pub(crate) top_side_bearings: Box<Vec<u16>>,
}

impl fmt::Display for VMTX {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl VMTX {
    pub(crate) fn new<R: BinaryReader>(
        file: &mut R,
        offest: u32,
        length: u32,
        number_of_hmetrics: u16,
        num_glyphs: u16,
    ) -> Result<Self, std::io::Error> {
        get_vmtx(file, offest, length, number_of_hmetrics, num_glyphs)
    }

    pub(crate) fn get_metrix(&self, i: usize) -> VerticalMetric {
        let v_metric = self.v_metrics.get(i);
        if v_metric.is_none() {
            return VerticalMetric {
                advance_height: 0,
                top_side_bearing: 0,
            };
        }
        v_metric.unwrap().clone()
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = "hmtx\n".to_string();
        let max_len = 10;
        for (i, v_metric) in self.v_metrics.iter().enumerate() {
            let advance_width = format!("{i} Advance Height {} ", v_metric.advance_height);
            string += &advance_width;
            let top_side_bearing = format!("Top Side Bearing {}\n", v_metric.top_side_bearing);
            string += &top_side_bearing;
            if max_len < i {
                break;
            }
        }
        for (i, top_side_bearing) in self.top_side_bearings.iter().enumerate() {
            let top_side_bearing = format!("{i} Left Side Bearing {}\n", top_side_bearing);
            string += &top_side_bearing;
            if max_len < i {
                break;
            }
        }
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct VerticalMetric {
    pub(crate) advance_height: u16,
    pub(crate) top_side_bearing: i16,
}

fn get_vmtx<R: bin_rs::reader::BinaryReader>(
    file: &mut R,
    offest: u32,
    _length: u32,
    number_of_hmetrics: u16,
    num_glyphs: u16,
) -> Result<VMTX, std::io::Error> {
    let file = file;
    file.seek(SeekFrom::Start(offest as u64))?;
    let mut v_metrics = Vec::new();
    for _ in 0..number_of_hmetrics {
        let advance_height = file.read_u16_be()?;
        let top_side_bearing = file.read_i16_be()?;
        v_metrics.push(VerticalMetric {
            advance_height,
            top_side_bearing,
        });
    }
    let mut top_side_bearings = Vec::new();
    let number = num_glyphs - number_of_hmetrics;
    for _ in 0..number {
        let top_side_bearing = file.read_u16_be()?;
        top_side_bearings.push(top_side_bearing);
    }
    Ok(VMTX {
        v_metrics: Box::new(v_metrics),
        top_side_bearings: Box::new(top_side_bearings),
    })
}
