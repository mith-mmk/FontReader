use std::{io::{Read, Seek, SeekFrom, Cursor}, fmt};
use byteorder::{BigEndian, ReadBytesExt};

#[derive(Debug, Clone)]
pub(crate) struct LOCA {
  pub(crate) offsets: Box<Vec<u32>>,
  number_of_print : usize,
}

impl fmt::Display for LOCA {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.to_string())
  }
}

impl LOCA{
  pub(crate) fn new<R:Read + Seek>(file: R, offest: u32, length: u32, num_glyphs: u16) -> Self {
    get_loca(file, offest, length, num_glyphs)
  }

  pub(crate) fn set_number_of_print(&mut self, value : usize) {
    self.number_of_print = value;
  }

  pub(crate) fn to_string(&self) -> String {
    let mut string = "loca\n".to_string();
    let length = if self.offsets.len() < self.number_of_print {
      self.offsets.len()
    } else {
      self.number_of_print
    };
    for i in 0..length {
      if i % 16 == 0 {
        string += "\n";
      }
      let offset = format!("{:3} Offset {:08x}\n",i, self.offsets[i]);
      string += &offset;
    }
    string
  }
}

fn get_loca<R:Read + Seek>(file: R, offest: u32, length: u32, num_glyphs: u16) -> LOCA {
  let size = length / num_glyphs as u32;
  let mut file = file;
  file.seek(SeekFrom::Start(offest as u64)).unwrap();
  let mut buf = vec![0; length as usize];
  file.read_exact(&mut buf).unwrap();
  let mut cursor = Cursor::new(buf);
  if size != 4 && size != 2 {
    panic!("Invalid size of loca table");
  }

  let mut offsets = Vec::new();
  for _ in 0..num_glyphs + 1 {
    let offset: u32 = if size == 2 {
      cursor.read_u16::<BigEndian>().unwrap() as u32 * 2
    } else {
      cursor.read_u32::<BigEndian>().unwrap()
    };
    offsets.push(offset);
  }

  LOCA {
    offsets: Box::new(offsets),
    number_of_print: 10,
  }

}