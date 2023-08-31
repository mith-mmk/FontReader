use std::{io::{Read, Seek, SeekFrom, Cursor}, fmt};
use byteorder::{BigEndian, ReadBytesExt};

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
  pub(crate) fn new<R:Read + Seek>(file: R, offest: u32, length: u32
    , number_of_hmetrics: u16,num_glyphs: u16) -> Self {
      get_hdmx(file, offest, length, number_of_hmetrics, num_glyphs)
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


fn get_hdmx<R: Read + Seek>(file: R, offest: u32, length: u32
                      , number_of_hmetrics: u16,num_glyphs: u16) -> HMTX {
  let mut file = file;
  file.seek(SeekFrom::Start(offest as u64)).unwrap();
  let mut buf = vec![0; length as usize];
  file.read_exact(&mut buf).unwrap();
  let mut cursor = Cursor::new(buf);
  let mut h_metrics = Vec::new();
  for _ in 0..number_of_hmetrics {
    let advance_width = cursor.read_u16::<BigEndian>().unwrap();
    let left_side_bearing = cursor.read_i16::<BigEndian>().unwrap();
    h_metrics.push(LongHorMetric {
      advance_width: advance_width,
      left_side_bearing: left_side_bearing,
    });
  }
  let mut left_side_bearings = Vec::new();
  let number = num_glyphs - number_of_hmetrics;
  for _ in 0..number {
    let left_side_bearing = cursor.read_u16::<BigEndian>().unwrap();
    left_side_bearings.push(left_side_bearing);
  }
  HMTX {
    h_metrics: Box::new(h_metrics),
    left_side_bearings: Box::new(left_side_bearings),
  }
}
