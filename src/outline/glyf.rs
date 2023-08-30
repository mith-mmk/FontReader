use core::num;
use std::{io::{Read, Seek, SeekFrom, Cursor}, fmt};
use byteorder::{BigEndian, ReadBytesExt};

use super::loca;
/*
int16	numberOfContours	If the number of contours is greater than or equal to zero, this is a simple glyph. If negative, this is a composite glyph — the value -1 should be used for composite glyphs.
int16	xMin	Minimum x for coordinate data.
int16	yMin	Minimum y for coordinate data.
int16	xMax	Maximum x for coordinate data.
int16	yMax	Maximum y for coordinate data.
*/

#[derive(Debug, Clone)]

pub(crate) struct GLYF {
  pub(crate) griphs: Box<Vec<Glyph>>,
}

#[derive(Debug, Clone)]
pub(crate) struct Glyph {
    pub number_of_contours: i16,
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
    pub glyphs: Box<Vec<u8>>,
}

impl fmt::Display for GLYF {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.to_string())
  }
}

impl GLYF {
  pub(crate) fn new<R:Read + Seek>(file: R, offest: u32, length: u32, loca: &Box<loca::LOCA>) -> Self {
    get_glyf(file, offest, length, &loca)
  }

  pub(crate) fn to_string(&self) -> String {
    let mut string = "glyf\n".to_string();

   let number_of_print = if self.griphs.len() < 10 {
      self.griphs.len()
    } else {
      10
    };
    for i in 0..number_of_print {
      let glyph = &self.griphs[i];
      let number_of_contours = format!("Number of Contours {}\n", glyph.number_of_contours);
      string += &number_of_contours;
      let x_min = format!("x_min {}\n", glyph.x_min);
      string += &x_min;
      let y_min = format!("y_min {}\n", glyph.y_min);
      string += &y_min;
      let x_max = format!("x_max {}\n", glyph.x_max);
      string += &x_max;
      let y_max = format!("y_max {}\n", glyph.y_max);
      string += &y_max;
      let glyphs = &glyph.glyphs;

      // dump buf
      for i in 0..glyphs.len() {
        if i % 16 == 0 {
          string += "\n";
        }
        let byte = format!("{:02x} ", glyphs[i]);
        string += &byte;
      }      
    }

    string

  }



}

#[derive(Debug, Clone)]
pub(crate) enum GlyphDescription {
  Symple(SympleGlyph),
  Composite(CompositeGlyph),
}
#[derive(Debug, Clone)]
pub(crate) struct SympleGlyph {
  pub end_pts_of_contours: Vec<u16>,
  pub instruction_length: u16,
  pub instructions: Vec<u8>,
  pub flags: Vec<u8>,
  pub x_coordinates: Vec<i16>,
  pub y_coordinates: Vec<i16>,
}

#[derive(Debug, Clone)]
pub(crate) struct CompositeGlyph {
  pub flags: Vec<u8>,
  pub glyph_index: Vec<u16>,
  pub argument1: Vec<i16>,
  pub argument2: Vec<i16>,
  pub scale: Vec<f32>,
  pub x_translate: Vec<f32>,
  pub y_translate: Vec<f32>,
}

fn get_glyf<R:Read + Seek>(file: R, offset: u32, length: u32, loca: &Box<loca::LOCA>) -> GLYF {
  let mut file = file;
  let loca = loca.clone();
  file.seek(SeekFrom::Start(offset as u64)).unwrap();
  let offsets = loca.offsets.clone();
  let mut glyphs = Vec::new();
  for i in 0..offsets.len() - 1 {
    let offset = offsets[i];
    let length = offsets[i + 1] - offset;
    let glyph = get_glyph(&mut file, offset, length);
    glyphs.push(glyph);
  }
  GLYF {
    griphs: Box::new(glyphs),
  }
}

fn get_glyph<R:Read + Seek>(file: &mut R, offset: u32, length: u32) -> Glyph {
  if length == 0 {
    return Glyph {
      number_of_contours: 0,
      x_min: 0,
      y_min: 0,
      x_max: 0,
      y_max: 0,
      glyphs: Box::new(Vec::new()),
    };
  }
  file.seek(SeekFrom::Start(offset as u64)).unwrap();

  let mut buf = vec![0u8; length as usize];
  file.read_exact(&mut buf).unwrap();
  let mut cursor = Cursor::new(buf);
  let number_of_contours = cursor.read_i16::<BigEndian>().unwrap();
  let x_min = cursor.read_i16::<BigEndian>().unwrap();
  let y_min = cursor.read_i16::<BigEndian>().unwrap();
  let x_max = cursor.read_i16::<BigEndian>().unwrap();
  let y_max = cursor.read_i16::<BigEndian>().unwrap();
  let griph;
  griph = cursor.into_inner();

  Glyph {
    number_of_contours,
    x_min,
    y_min,
    x_max,
    y_max,
    glyphs: Box::new(griph),
  }
}
