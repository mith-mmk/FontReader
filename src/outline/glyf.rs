use core::num;
use std::{io::{Read, Seek, SeekFrom, Cursor, BufRead}, fmt, os::raw, vec};
use byteorder::{BigEndian, ReadBytesExt};

use crate::requires::cmap::CmapHighByteEncoding;

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
    pub glyphs: Box<Vec<u8>>,
    pub offset: u32,
    pub length: u32,
}

#[derive(Debug, Clone)]
pub struct ParsedGlyph {
  pub number_of_contours: i16,
  pub x_min: i16,
  pub y_min: i16,
  pub x_max: i16,
  pub y_max: i16,
  pub offset: u32,
  pub length: u32,
  pub end_pts_of_contours: Vec<usize>,
  pub instructions: Vec<u8>,
  pub flags: Vec<u8>,
  pub xs: Vec<i16>,
  pub ys: Vec<i16>,
  pub on_curves: Vec<bool>,
}


impl fmt::Display for Glyph {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.to_string())
  }
}

impl Glyph {
  pub fn parse(&self) -> ParsedGlyph {


    if self.length < 10 {
      return ParsedGlyph {
        number_of_contours: 0,
        x_min: 0,
        y_min: 0,
        x_max: 0,
        y_max: 0,
        offset: self.offset,
        length: self.length,
        end_pts_of_contours: Vec::new(),
        instructions: Vec::new(),
        flags: Vec::new(),
        xs: Vec::new(),
        ys: Vec::new(),
        on_curves: Vec::new(),
      }
    }
    let buf = self.glyphs.clone();
    let mut offset = 0;
    let high_byte = buf[offset] as u16;
    let low_byte = buf[offset + 1] as u16;
    let number_of_contours = ((high_byte << 8) + low_byte) as i16;
    offset += 2;
    let high_byte = buf[offset] as u16;
    let low_byte = buf[offset + 1] as u16;
    let x_min = ((high_byte << 8) + low_byte) as i16;
    offset += 2;
    let high_byte = buf[offset] as u16;
    let low_byte = buf[offset + 1] as u16;
    let y_min = ((high_byte << 8) + low_byte) as i16;
    offset += 2;
    let high_byte = buf[offset] as u16;
    let low_byte = buf[offset + 1] as u16;
    let x_max = ((high_byte << 8) + low_byte) as i16;
    offset += 2;
    let high_byte = buf[offset] as u16;
    let low_byte = buf[offset + 1] as u16;
    let y_max = ((high_byte << 8) + low_byte) as i16;
    offset += 2;
    

    let mut instructions = Vec::new();
    let mut flags = Vec::new();
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    let mut on_curves = Vec::new();
    let mut coutour = Vec::new();

    if number_of_contours >= 0 {
      for _ in 0..number_of_contours as usize {
        let high_byte = self.glyphs[offset] as u16;
        let low_byte = self.glyphs[offset + 1] as u16;
        offset += 2;
        let number = (high_byte << 8) + low_byte;
        coutour.push(number as usize);
      } 
      let last_end_pts_of_contours = coutour[number_of_contours as usize as usize - 1] + 1;
      let high_byte = self.glyphs[offset] as u16;
      let low_byte = self.glyphs[offset + 1] as u16;
      let instruction_length = (high_byte << 8) + low_byte;
      offset += 2;

      for _ in 0..instruction_length {
        let instruction = self.glyphs[offset];
        instructions.push(instruction);
        offset += 1;
      }
      let mut i = 0;
        while i < last_end_pts_of_contours {
        // flags 
        let flag = self.glyphs[offset];
        offset += 1;
        flags.push(flag);
        let mut repeat = 0;
        if flag & 0x08 != 0 {
          repeat = self.glyphs[offset];
          offset += 1;
        }
        for _ in 0..repeat {
          flags.push(flag);
          i += 1;
        }
        i += 1;
      }
      for flag in flags.iter() {
        let on_curve = flag & 0x01 != 0;
        on_curves.push(on_curve);
      }

      i = 0;
      for flag in flags.iter() {
        let mut x = 0;
        if flag & 0x2 != 0 {
          let byte = self.glyphs[offset];
          offset += 1;
          if flag & 0x10 != 0 {
            x += byte as i16;
          } else {
            x -= byte as i16;
          }
        } else if flag & 0x10 == 0 {
          let hi_byte = self.glyphs[offset] as u16;
          let lo_byte = self.glyphs[offset + 1] as u16;
          offset += 2;
          let byte = (hi_byte << 8) + lo_byte;
          x = byte as i16;
        }
        xs.push(x);          
      }
      for flag in flags.iter() {
        let mut y = 0;
        if  flag & 0x4 != 0 {
          let byte = self.glyphs[offset];
          offset += 1;
          if flag & 0x20 != 0 {
            y += byte as i16;
          } else {
            y -= byte as i16;
          }
        } else if flag & 0x20 == 0 {
          let hi_byte = self.glyphs[offset] as u16;
          let lo_byte = self.glyphs[offset + 1] as u16;
          offset += 2;
          let byte = (hi_byte << 8) + lo_byte;
          y = byte as i16;
        }
        ys.push(y);
      }
    } else {
      // TODO: composite glyph
    }

    ParsedGlyph {
      number_of_contours,
      x_min,
      y_min,
      x_max,
      y_max,
      offset: self.offset,
      length: self.length,
      end_pts_of_contours: coutour,
      instructions,
      flags,
      xs,
      ys,
      on_curves,
    }
  }

  pub fn to_string(&self) -> String {
    let parsed = self.parse();
    let mut string = "glyph\n".to_string();

    string += &format!("Number of Contours {}\n", parsed.number_of_contours);
    string += &format!("x_min {}\n", parsed.x_min);
    string += &format!("x_max {}\n", parsed.x_max);
    string += &format!("y_max {}\n", parsed.y_max);
    string += &format!("y_min {}\n", parsed.y_min);
    string += &format!("offset {}\n", parsed.offset);
    string += &format!("length {}\n", parsed.length);


    let length = self.glyphs.len();
    string += &format!("buffer length {}\n", length);

    if length == 0 {
      string += "empty glyph\n";
      return string;
    }

    let mut pos = 0;
    for i in 0..parsed.end_pts_of_contours.len() {
      string += &format!("{} end_pts_of_contours {}\n",i, parsed.end_pts_of_contours[i]);
    }
    let mut x = 0;
    let mut y = 0;

    for i in 0..parsed.flags.len() {
      let dx = parsed.xs[i];
      let dy = parsed.ys[i];
      x += dx;
      y += dy;
      string += &format!("{:2} flag {} {:08b} {} {} {} {}\n",i, pos, parsed.flags[i],x, y, dx, dy);
      if i >= parsed.end_pts_of_contours[pos] {
        pos += 1;
      }

    }
    

    string


  }


}


impl fmt::Display for GLYF {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.to_string())
  }
}

impl GLYF {
  pub(crate) fn new<R:Read + Seek>(file: R, offset: u32, length: u32, loca: &loca::LOCA) -> Self {
    get_glyf(file, offset, length, &loca)
  }

  pub fn get_glyph(&self, index: usize) -> Option<&Glyph> {
    self.griphs.get(index)
  }


  pub(crate) fn to_string(&self) -> String {
    let mut string = "glyf\n".to_string();
    string += &format!("number of glyphs {}\n", self.griphs.len());
    let max_number = 10;
    for (i, glyph) in self.griphs.iter().enumerate() {
      string += &format!("glyph {}\n", i);
      string += &glyph.to_string();
      if max_number < i {
        break;
      }
    }
    string
  }
}



fn get_glyf<R:Read + Seek>(file: R, offset: u32, length: u32, loca: &loca::LOCA) -> GLYF {
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
      offset: offset,
      length: length,
      glyphs: Box::new(Vec::new()),
    };
  }

  let mut buf = vec![0u8; length as usize];
  file.read_exact(&mut buf).unwrap();
  let cursor = Cursor::new(buf);
  let griph= cursor.into_inner();

  Glyph {
    glyphs: Box::new(griph),
    offset,
    length,
  }
}
