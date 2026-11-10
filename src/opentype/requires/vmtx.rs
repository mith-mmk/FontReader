use bin_rs::reader::BinaryReader;
use std::{fmt, io::SeekFrom};

// vmtx table Font vertical metrics

#[derive(Debug, Clone)]
pub(crate) struct Vmtx {
    pub(crate) v_metrics: Vec<VerticalMetric>,
}

impl fmt::Display for Vmtx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format_table())
    }
}

impl Vmtx {
    pub(crate) fn new<R: BinaryReader>(
        file: &mut R,
        offest: u32,
        length: u32,
        number_of_vmetrics: u16,
        num_glyphs: u16,
    ) -> Result<Self, std::io::Error> {
        get_vmtx(file, offest, length, number_of_vmetrics, num_glyphs)
    }

    pub(crate) fn get_metrix(&self, i: usize) -> VerticalMetric {
        let v_metric = self.v_metrics.get(i);
        if v_metric.is_none() {
            let advance_height = self
                .v_metrics
                .last()
                .map(|metric| metric.advance_height)
                .unwrap_or(0);
            return VerticalMetric {
                advance_height,
                top_side_bearing: 0,
            };
        }
        v_metric.unwrap().clone()
    }

    fn format_table(&self) -> String {
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
    number_of_vmetrics: u16,
    num_glyphs: u16,
) -> Result<Vmtx, std::io::Error> {
    file.seek(SeekFrom::Start(offest as u64))?;
    let mut v_metrics = Vec::new();
    for _ in 0..number_of_vmetrics {
        let advance_height = file.read_u16_be()?;
        let top_side_bearing = file.read_i16_be()?;
        v_metrics.push(VerticalMetric {
            advance_height,
            top_side_bearing,
        });
    }
    let advance_height = v_metrics
        .last()
        .map(|metric| metric.advance_height)
        .unwrap_or(0);
    let number = num_glyphs.saturating_sub(number_of_vmetrics);
    for _ in 0..number {
        let top_side_bearing = file.read_i16_be()?;
        v_metrics.push(VerticalMetric {
            advance_height,
            top_side_bearing,
        });
    }
    Ok(Vmtx { v_metrics })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bin_rs::reader::BytesReader;

    #[test]
    fn vmtx_allows_zero_vertical_metrics() {
        let buffer = [0x00, 0x04, 0xFF, 0xFD];
        let mut reader = BytesReader::new(&buffer);
        let vmtx = Vmtx::new(&mut reader, 0, buffer.len() as u32, 0, 2).expect("parse vmtx");

        assert_eq!(vmtx.v_metrics.len(), 2);
        assert_eq!(vmtx.get_metrix(0).advance_height, 0);
        assert_eq!(vmtx.get_metrix(0).top_side_bearing, 4);
        assert_eq!(vmtx.get_metrix(1).top_side_bearing, -3);
        assert_eq!(vmtx.get_metrix(99).advance_height, 0);
        assert_eq!(vmtx.get_metrix(99).top_side_bearing, 0);
    }
}
