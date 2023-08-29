use std::io::{Read, Seek, SeekFrom, Cursor};
use byteorder::{BigEndian, ReadBytesExt};

#[derive(Debug, Clone)]
pub(crate) struct HMTX {
  pub(crate) h_metrics: Box<Vec<LongHorMetric>>,
  pub(crate) left_side_bearings: Box<Vec<u16>>,
}

#[derive(Debug, Clone)]
pub(crate) struct LongHorMetric {
  pub(crate) advance_width: u16,
  pub(crate) left_side_bearing: i16,
} 


pub(crate) fn get_hdmx<R: Read + Seek>(file: R, offest: u32, length: u32
                      , number_of_hmetrics: u32,num_glyphs: u32) -> HMTX {
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
